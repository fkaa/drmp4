use h264_reader::{
    nal::{pps::PicParameterSet, slice::SliceHeader, sps::SeqParameterSet, NalHeader, UnitType},
    rbsp::{decode_nal, BitRead, BitReader},
    Context,
};
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Range;

#[derive(Default)]
pub struct H264Stream {
    ctx: Context,
    last_frame_num: u16,
    nal_units: Vec<(NalHeader, Range<u64>)>,
}

impl H264Stream {
    pub fn nal_units(&self) -> &[(NalHeader, Range<u64>)] {
        &self.nal_units
    }

    pub fn process_stream<R: BufRead + Seek>(&mut self, reader: &mut R) -> anyhow::Result<()> {
        'outer: loop {
            let buffer_pos = reader.stream_position()?;
            let mut buffer = reader.fill_buf()?;
            let mut buffer_offset = 0;
            let buffer_len = buffer.len().min(1024);

            if buffer_len <= 4 {
                return Ok(());
            }

            loop {
                let potential_nal_unit = locate_nal_unit(&buffer[buffer_offset..], 10000000);
                match potential_nal_unit {
                    Some((offset, header, length)) => {
                        println!(
                            "{}, {}, {}, {}, {:?}",
                            buffer_pos + buffer_offset as u64 + offset as u64,
                            buffer_len,
                            buffer_offset as u64,
                            length,
                            header
                        );
                        if !self.is_nal_believable(
                            header,
                            length,
                            buffer_pos,
                            &buffer[(buffer_offset + offset)
                                ..(buffer_offset + offset + length as usize).min(buffer.len())],
                        ) {
                            buffer_offset += offset - 3;
                            continue;
                        }

                        let nal_start = buffer_pos + buffer_offset as u64 + offset as u64;
                        let nal_end = nal_start + length as u64;
                        self.nal_units.push((header, nal_start..nal_end));

                        println!("Seeking to nal end {}", nal_end);

                        reader.seek(SeekFrom::Start(nal_end))?;
                        continue 'outer;
                    }
                    _ => {
                        reader.seek(SeekFrom::Current(buffer_len as i64 - 4))?;
                        println!("Seeking to {}", buffer_pos + buffer_len as u64 - 4);
                        continue 'outer;
                    }
                }
            }
        }
    }

    pub fn is_nal_believable(
        &mut self,
        header: NalHeader,
        len: u32,
        offset: u64,
        nal: &[u8],
    ) -> bool {
        if len < 4 {
            dbg!(len);
            return false;
        }

        if u8::from(header) == 0x01 {
            println!(
                "{}, {:?}, {:02x}",
                offset,
                header.nal_unit_type(),
                u8::from(header)
            );
        }

        match header.nal_unit_type() {
            UnitType::SeqParameterSet if len < 512 => {
                let Ok(nal) = decode_nal(nal) else {
                return false;
                };
                let mut reader = BitReader::new(Cursor::new(nal));
                if let Ok(sps) = SeqParameterSet::from_bits(reader) {
                    self.ctx.put_seq_param_set(sps);
                    true
                } else {
                    false
                }
            }
            UnitType::PicParameterSet if len < 512 => {
                let Ok(nal) = decode_nal(nal) else {
                return false;
                };
                let mut reader = BitReader::new(Cursor::new(nal));
                if let Ok(pps) = PicParameterSet::from_bits(&self.ctx, reader) {
                    self.ctx.put_pic_param_set(pps);
                    true
                } else {
                    false
                }
            }
            UnitType::AccessUnitDelimiter if header.nal_ref_idc() == 0 && len < 1024 * 5 => true,
            UnitType::SEI if header.nal_ref_idc() == 0 && len < 1024 * 5 => true,
            UnitType::FillerData if header.nal_ref_idc() == 0 && len < 1024 * 1024 * 2 => {
                nal.iter().filter(|i| **i == 0xff).count() > nal.len() - (nal.len() / 10)
            }
            UnitType::SliceLayerWithoutPartitioningIdr
                if header.nal_ref_idc() > 0 && len < 1024 * 1024 * 5 =>
            {
                self.is_slice_header_believeable(header, nal)
            }
            UnitType::SliceLayerWithoutPartitioningNonIdr if len < 1024 * 500 => {
                /*if header.nal_ref_idc() == 1 {
                    return false;
                }*/
                self.is_slice_header_believeable(header, nal)
            }
            _ => false,
        }
    }

    fn is_slice_header_believeable(&mut self, header: NalHeader, nal_unit: &[u8]) -> bool {
        let mut reader = BitReader::new(Cursor::new(&nal_unit[1..]));
        let slice_header = SliceHeader::from_bits(&self.ctx, &mut reader, header);
        match slice_header {
            Ok((header, _, _)) => {
                if header.frame_num < self.last_frame_num && header.frame_num != 0 {
                    return false;
                }
                // println!("frame-num: {}", header.frame_num);

                self.last_frame_num = header.frame_num;
                true
            }
            Err(e) => {
                println!("{:?}: {}, {:?}", header.nal_unit_type(), nal_unit.len(), e);

                while let None = reader.reader() {
                    reader.read_bool("");
                }

                let cursor = reader.reader().unwrap();
                // println!("cursor {}", cursor.position());

                // cursor.position() > 2
                false
            }
        }
    }
}

fn locate_nal_unit(buf: &[u8], len_threshold: u32) -> Option<(usize, NalHeader, u32)> {
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
