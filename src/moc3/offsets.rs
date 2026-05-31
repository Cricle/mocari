use crate::{Error, Result};

use super::{Endianness, Moc3Header};

const OFFSET_TABLE_START: usize = 0x40;
const CONFIRMED_OFFSET_COUNT: usize = 2;
const U32_SIZE: usize = 4;
const CONFIRMED_TABLE_END: usize = OFFSET_TABLE_START + CONFIRMED_OFFSET_COUNT * U32_SIZE;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Moc3SectionOffsets {
    count_info_offset: u32,
    canvas_info_offset: u32,
}

impl Moc3SectionOffsets {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let table_len = CONFIRMED_OFFSET_COUNT * U32_SIZE;
        if bytes.len() < OFFSET_TABLE_START + table_len {
            return Err(invalid_offsets("section offset table is incomplete"));
        }

        let count_info_offset = read_u32(bytes, OFFSET_TABLE_START, header.endianness());
        let canvas_info_offset =
            read_u32(bytes, OFFSET_TABLE_START + U32_SIZE, header.endianness());

        validate_offset(bytes, count_info_offset, "count info")?;
        validate_offset(bytes, canvas_info_offset, "canvas info")?;

        Ok(Self {
            count_info_offset,
            canvas_info_offset,
        })
    }

    pub fn count_info_offset(&self) -> u32 {
        self.count_info_offset
    }

    pub fn canvas_info_offset(&self) -> u32 {
        self.canvas_info_offset
    }
}

fn read_u32(bytes: &[u8], offset: usize, endianness: Endianness) -> u32 {
    let raw = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];

    match endianness {
        Endianness::Little => u32::from_le_bytes(raw),
        Endianness::Big => u32::from_be_bytes(raw),
    }
}

fn validate_offset(bytes: &[u8], offset: u32, name: &'static str) -> Result<()> {
    let offset = usize::try_from(offset)
        .map_err(|_| invalid_offsets(format!("{name} offset does not fit in platform usize")))?;

    if offset >= bytes.len() {
        return Err(invalid_offsets(format!("{name} offset is outside file")));
    }

    if offset < CONFIRMED_TABLE_END {
        return Err(invalid_offsets(format!(
            "{name} offset points into the header or confirmed offset table"
        )));
    }

    if offset % U32_SIZE != 0 {
        return Err(invalid_offsets(format!(
            "{name} offset is not 4-byte aligned"
        )));
    }

    Ok(())
}

fn invalid_offsets(message: impl Into<String>) -> Error {
    Error::InvalidMoc3 {
        message: message.into(),
    }
}
