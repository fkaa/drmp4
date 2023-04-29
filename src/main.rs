use memmap::Mmap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Range;

use h264_reader::{
    nal::{pps::PicParameterSet, slice::SliceHeader, sps::SeqParameterSet, NalHeader, UnitType},
    rbsp::{decode_nal, BitRead, BitReader},
    Context,
};

mod h264;

use h264::*;

fn copy_file_slice<R: Read + Seek, W: Write>(
    mut from: &mut R,
    slice: Range<u64>,
    mut to: &mut W,
) -> anyhow::Result<()> {
    from.seek(SeekFrom::Start(slice.start))?;

    let mut len = (slice.end - slice.start) as usize;
    let mut buffer = [0u8; 4096];

    while len > 0 {
        let read_bytes = from.read(&mut buffer[..len.min(4096)])?;
        if read_bytes == 0 {
            anyhow::bail!("EOF!");
        }

        len -= read_bytes;

        to.write_all(&buffer[..read_bytes])?;
    }

    Ok(())
}

fn get_start_code_for_nal(header: &NalHeader) -> &'static [u8] {
    match header.nal_unit_type() {
        UnitType::SeqParameterSet
        | UnitType::PicParameterSet
        | UnitType::SEI
        | UnitType::SliceLayerWithoutPartitioningNonIdr => &[0, 0, 0, 1],
        _ => &[0, 0, 1],
    }
}

use owo_colors::{OwoColorize, Style};

fn get_style(header: &NalHeader) -> Style {
    match header.nal_unit_type() {
        UnitType::SliceLayerWithoutPartitioningIdr => Style::new().on_green(),
        UnitType::SliceLayerWithoutPartitioningNonIdr => Style::new().on_bright_magenta(),
        UnitType::FillerData => Style::new().on_bright_black(),
        _ => Style::new(),
    }
}

fn main() {
    let mut stream = H264Stream::default();

    let file = File::open("/home/tmtu/working.mp4").unwrap();
    let mut reader = BufReader::new(file);
    let buf = reader.fill_buf().unwrap();
    let Some(mdat) = twoway::find_bytes(&buf, b"mdat") else {
        return;
    };
    reader.seek(SeekFrom::Start(mdat as u64 + 4)).unwrap();

    stream.process_stream(&mut reader).unwrap();

    let mut h264 = File::create("/home/tmtu/video.h264").unwrap();

    for (nal_header, nal_unit) in stream.nal_units() {
        let nal_style = get_style(nal_header);
        /*println!(
            "{:02x} {:>50?} (Len={}, Offset={})",
            u8::from(*nal_header).style(nal_style),
            nal_header.nal_unit_type().style(nal_style),
            nal_unit.end - nal_unit.start,
            nal_unit.start,
        );*/
        h264.write_all(get_start_code_for_nal(nal_header)).unwrap();
        copy_file_slice(&mut reader, nal_unit.clone(), &mut h264).unwrap();
    }

    /*let file = File::open("/home/tmtu/working.mp4").unwrap();
    let mmap = unsafe { Mmap::map(&file).unwrap() };


    let mut h264 = File::create("/home/tmtu/video.h264").unwrap();
    let mut aac = File::create("/home/tmtu/audio.aac").unwrap();

    let mut i = mdat + 4;

    println!("mdat: {}", mdat);
    let mut stream = H264Stream::default();

    let mut threshold = 2000;
    let mut last_nal_start = i;
    let mut last_nal_end = i;

    while let Some((offset, hdr, len)) = locate_nal_unit(&mmap[i..], threshold) {
        i += offset;

        if i + len as usize > mmap.len() {
            continue;
        }
        let nal_unit = &mmap[i..(i + len as usize)];
        let b = stream.is_nal_believable(hdr, len, nal_unit);
        println!(
            "{}: {} {} ({:?}), {}, Length={len}, StreamPos={:?}, {}",
            i,
            b,
            hdr.nal_unit_type().id(),
            hdr.nal_unit_type(),
            hdr.nal_ref_idc(),
            h264.stream_position(),
            i - last_nal_end
        );
        if b {
            println!("{}", i - last_nal_end);

            if i - last_nal_end > 4 {
                aac.write_all(&mmap[last_nal_end..(i - 4)]).unwrap();
            }
            last_nal_start = i;
            //if threshold == 2000 {
            h264.write_all(&[0, 0, 1]).unwrap();
            h264.write_all(nal_unit).unwrap();
            //} else {
            //   write_nal_with_emulation(&mut h264, &nal_unit);
            //}

            // println!("{:?}", twoway::find_bytes(&nal_unit, &[0, 0, 1]));

            i += len as usize;
            last_nal_end = i;
        } else {
            i -= 3;
        }

        if hdr.nal_unit_type() == UnitType::PicParameterSet {
            threshold = 1024 * 1024 * 3;
        }

        if i > 900000 {
            break;
        }
    }*/
}

fn write_nal_with_emulation(file: &mut File, mut nal_unit: &[u8]) {
    use aho_corasick::AhoCorasick;

    let patterns = &[&[0, 0, 1], &[0, 0, 2], &[0, 0, 3]];
    let ac = AhoCorasick::new(patterns).unwrap();

    file.write_all(&[0, 0, 0, 1]).unwrap();
    while let Some(mat) = ac.find(nal_unit) {
        println!(
            "{:?}, {:?}, {}",
            mat,
            &nal_unit[mat.start()..mat.end()],
            nal_unit[mat.end() - 1]
        );

        let start = &nal_unit[..mat.start()];
        file.write_all(start).unwrap();

        let emulated = [0, 0, 3, nal_unit[mat.end() - 1]];
        file.write_all(&emulated[..]).unwrap();

        let rest = &nal_unit[mat.end()..];

        nal_unit = rest;

        //println!("{}", start.len() + emulated.len());
    }

    if !nal_unit.is_empty() {
        //println!("{}", nal_unit.len());
        file.write_all(nal_unit).unwrap();
    }
}

pub fn locate_nal_unit(buf: &[u8], len_threshold: u32) -> Option<(usize, NalHeader, u32)> {
    for i in 0..buf.len() {
        let Ok(header) = NalHeader::new(buf[i]) else {
    continue;
        };

        if let UnitType::Unspecified(_) = header.nal_unit_type() {
            continue;
        }

        let len = if i >= 4 {
            u32::from_be_bytes(buf[i - 4..i].try_into().unwrap())
        } else {
            continue;
        };

        if len > len_threshold {
            continue;
        }

        // println!("{:02x}", buf[i]);

        return Some((i, header, len));
    }

    None
}
