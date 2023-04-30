use bitter::{BitReader, LittleEndianReader};
use std::error::Error;
use std::fmt;

const ID_SCE: u8 = 0;
const ID_CPE: u8 = 1;
const ID_CCE: u8 = 2;
const ID_LFE: u8 = 3;
const ID_DSE: u8 = 4;
const ID_PCE: u8 = 5;
const ID_FIL: u8 = 6;
const ID_END: u8 = 7;

const ONLY_LONG_SEQUENCE: u8 = 0;
const LONG_START_SEQUENCE: u8 = 1;
const EIGHT_SHORT_SEQUENCE: u8 = 2;
const LONG_STOP_SEQUENCE: u8 = 3;

#[derive(Debug)]
enum AacError {
    Empty,
}

impl fmt::Display for AacError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AacError::Empty => write!(formatter, "Not enough bits")?,
        }

        Ok(())
    }
}

impl Error for AacError {}

fn raw_data_block(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    loop {
        let len = require_bits(bits, 3)?;
        let id = bits.peek(3) as u8;
        bits.consume(3);

        match id {
            self::ID_SCE => single_channel_element(bits)?,
            self::ID_CPE => {}
            self::ID_CCE => {}
            self::ID_LFE => {}
            self::ID_DSE => {}
            self::ID_PCE => {}
            self::ID_FIL => {}
            self::ID_END => break,
            _ => unreachable!(),
        }
    }

    while !bits.byte_aligned() {
        bits.consume(1);
    }

    todo!()
}

fn single_channel_element(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    let tag = bits.read_bits(4).ok_or(AacError::Empty)?;
    Ok(())
}

fn individual_channel_stream(
    bits: &mut LittleEndianReader,
    common_window: u8,
    scale_flags: u8,
) -> anyhow::Result<()> {
    let global_gain = bits.read_bits(8).ok_or(AacError::Empty)?;

    if common_window == 0 && scale_flags == 0 {
        ics_info(bits)?;
    }
    section_data(bits, todo!())?;
    scale_factor_data(bits)?;

    if scale_flags == 0 {
        let pulse_data_present = bits.read_bit().ok_or(AacError::Empty)?;
        if pulse_data_present {
            pulse_data(bits)?;
        }
        let tns_data_present = bits.read_bit().ok_or(AacError::Empty)?;
        if pulse_data_present {
            tns_data(bits)?;
        }
        let gain_control_data_present = bits.read_bit().ok_or(AacError::Empty)?;
        if pulse_data_present {
            gain_control_data(bits)?;
        }
    }

    spectral_data(bits)?;
    Ok(())
}

fn ics_info(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn section_data(bits: &mut LittleEndianReader, window_sequence: u8) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn scale_factor_data(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn pulse_data(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn tns_data(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn gain_control_data(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn spectral_data(bits: &mut LittleEndianReader) -> anyhow::Result<()> {
    todo!();

    Ok(())
}

fn window_sequence(window_sequence: u8) {
    let num_windows;
    let num_window_groups;

    match window_sequence {
        self::ONLY_LONG_SEQUENCE => {
            num_windows = 1;
            num_window_groups = 1;
        }
        _ => unreachable!(),
    }
}

/*
switch (window_sequence) {
case ONLY_LONG_SEQUENCE:
case LONG_START_SEQUENCE:
case LONG_STOP_SEQUENCE:
num_windows = 1;
num_window_groups = 1;
window_group_length[num_window_groups-1] = 1;
num_swb = num_swb_long_window[fs_index];
/* preparation of sect_sfb_offset for long blocks */
/* also copy the last value! */
for(i = 0; i < max_sfb + 1; i++) {
sect_sfb_offset[0][i] = swb_offset_long_window[fs_index][i];
swb_offset[i] = swb_offset_long_window[fs_index][i];
}
break;
case EIGHT_SHORT_SEQUENCE:
num_windows = 8;
num_window_groups = 1;
window_group_length[num_window_groups-1] = 1;
num_swb = num_swb_short_window[fs_index];
for (i = 0; i < num_swb_short_window[fs_index] + 1; i++)
swb_offset[i] = swb_offset_short_window[fs_index][i];
for (i = 0; i < num_windows-1; i++) {
if (bit_set(scale_factor_grouping,6-i)) == 0) {
num_window_groups += 1;
window_group_length[num_window_groups-1] = 1;
}
else {
window_group_length[num_window_groups-1] += 1;
}
}
/* preparation of sect_sfb_offset for short blocks */
for (g = 0; g < num_window_groups; g++) {
sect_sfb = 0;
offset = 0;
for (i = 0; i < max_sfb; i++) {
width = swb_offset_short_window[fs_index][i+1] -
swb_offset_short_window[fs_index][i];
width *= window_group_length[g];
sect_sfb_offset[g][sect_sfb++] = offset;
offset += width;
}
sect_sfb_offset[g][sect_sfb] = offset;
}
break;
default:
break;
}
 */

fn require_bits(bits: &mut LittleEndianReader, len: u32) -> anyhow::Result<u32> {
    let bits_available = bits.refill_lookahead();
    if bits_available < len {
        anyhow::bail!("Not enough bits available");
    }

    Ok(bits_available)
}
