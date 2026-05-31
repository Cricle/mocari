use crate::{Error, Result};

use super::{Endianness, Moc3CountInfo, Moc3Header, Moc3SectionOffsets};

const TEXTURE_INDICES_SLOT: usize = 41;
const DRAWABLE_FLAGS_SLOT: usize = 42;
const VERTEX_COUNTS_SLOT: usize = 43;
const UV_BEGIN_INDICES_SLOT: usize = 44;
const POSITION_INDEX_BEGIN_INDICES_SLOT: usize = 45;
const POSITION_INDEX_COUNTS_SLOT: usize = 46;
const MASK_BEGIN_INDICES_SLOT: usize = 47;
const MASK_COUNTS_SLOT: usize = 48;
const UV_XYS_SLOT: usize = 78;
const POSITION_INDICES_SLOT: usize = 79;
const DRAWABLE_MASKS_SLOT: usize = 80;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Moc3ArtMeshInfo {
    texture_index: i32,
    drawable_flags: u8,
    position_index_count: i32,
    uv_begin_index: i32,
    position_index_begin_index: i32,
    vertex_count: i32,
    mask_begin_index: i32,
    mask_count: i32,
}

impl Moc3ArtMeshInfo {
    pub fn new(
        texture_index: i32,
        drawable_flags: u8,
        position_index_count: i32,
        uv_begin_index: i32,
        position_index_begin_index: i32,
        vertex_count: i32,
        mask_begin_index: i32,
        mask_count: i32,
    ) -> Self {
        Self {
            texture_index,
            drawable_flags,
            position_index_count,
            uv_begin_index,
            position_index_begin_index,
            vertex_count,
            mask_begin_index,
            mask_count,
        }
    }

    pub fn texture_index(&self) -> i32 {
        self.texture_index
    }

    pub fn drawable_flags(&self) -> u8 {
        self.drawable_flags
    }

    pub fn position_index_count(&self) -> i32 {
        self.position_index_count
    }

    pub fn uv_begin_index(&self) -> i32 {
        self.uv_begin_index
    }

    pub fn position_index_begin_index(&self) -> i32 {
        self.position_index_begin_index
    }

    pub fn vertex_count(&self) -> i32 {
        self.vertex_count
    }

    pub fn mask_begin_index(&self) -> i32 {
        self.mask_begin_index
    }

    pub fn mask_count(&self) -> i32 {
        self.mask_count
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3ArtMeshes {
    meshes: Vec<Moc3ArtMeshInfo>,
    uv_xys: Vec<f32>,
    position_indices: Vec<i16>,
    drawable_masks: Vec<i32>,
}

impl Moc3ArtMeshes {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let art_mesh_count = to_usize(counts.art_meshes(), "art mesh count")?;

        let texture_indices = read_i32_section(
            bytes,
            &offsets,
            TEXTURE_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let drawable_flags = read_u8_section(bytes, &offsets, DRAWABLE_FLAGS_SLOT, art_mesh_count)?;
        let vertex_counts = read_i32_section(
            bytes,
            &offsets,
            VERTEX_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let uv_begin_indices = read_i32_section(
            bytes,
            &offsets,
            UV_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let position_index_begin_indices = read_i32_section(
            bytes,
            &offsets,
            POSITION_INDEX_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let position_index_counts = read_i32_section(
            bytes,
            &offsets,
            POSITION_INDEX_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let mask_begin_indices = read_i32_section(
            bytes,
            &offsets,
            MASK_BEGIN_INDICES_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let mask_counts = read_i32_section(
            bytes,
            &offsets,
            MASK_COUNTS_SLOT,
            art_mesh_count,
            header.endianness(),
        )?;
        let uv_xys = read_f32_section(
            bytes,
            &offsets,
            UV_XYS_SLOT,
            to_usize(counts.uvs(), "uv count")?,
            header.endianness(),
        )?;
        let position_indices = read_i16_section(
            bytes,
            &offsets,
            POSITION_INDICES_SLOT,
            to_usize(counts.position_indices(), "position index count")?,
            header.endianness(),
        )?;
        let drawable_masks = read_i32_section(
            bytes,
            &offsets,
            DRAWABLE_MASKS_SLOT,
            to_usize(counts.drawable_masks(), "drawable mask count")?,
            header.endianness(),
        )?;

        let mut meshes = Vec::with_capacity(art_mesh_count);
        for index in 0..art_mesh_count {
            let mesh = Moc3ArtMeshInfo::new(
                texture_indices[index],
                drawable_flags[index],
                position_index_counts[index],
                uv_begin_indices[index],
                position_index_begin_indices[index],
                vertex_counts[index],
                mask_begin_indices[index],
                mask_counts[index],
            );
            validate_mesh_ranges(
                index,
                mesh,
                uv_xys.len(),
                position_indices.len(),
                drawable_masks.len(),
            )?;
            meshes.push(mesh);
        }

        Ok(Self {
            meshes,
            uv_xys,
            position_indices,
            drawable_masks,
        })
    }

    pub fn meshes(&self) -> &[Moc3ArtMeshInfo] {
        &self.meshes
    }

    pub fn uv_xys(&self) -> &[f32] {
        &self.uv_xys
    }

    pub fn position_indices(&self) -> &[i16] {
        &self.position_indices
    }

    pub fn drawable_masks(&self) -> &[i32] {
        &self.drawable_masks
    }

    pub fn art_mesh_uvs(&self, index: usize) -> Option<&[f32]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.uv_begin_index).ok()?;
        let len = usize::try_from(mesh.vertex_count).ok()?.checked_mul(2)?;
        self.uv_xys.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_position_indices(&self, index: usize) -> Option<&[i16]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.position_index_begin_index).ok()?;
        let len = usize::try_from(mesh.position_index_count).ok()?;
        self.position_indices.get(start..start.checked_add(len)?)
    }

    pub fn art_mesh_masks(&self, index: usize) -> Option<&[i32]> {
        let mesh = self.meshes.get(index)?;
        let start = usize::try_from(mesh.mask_begin_index).ok()?;
        let len = usize::try_from(mesh.mask_count).ok()?;
        self.drawable_masks.get(start..start.checked_add(len)?)
    }
}

fn read_i32_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<i32>> {
    read_section(bytes, offsets, slot, count, 4, |bytes, offset| {
        let raw = [
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ];
        match endianness {
            Endianness::Little => i32::from_le_bytes(raw),
            Endianness::Big => i32::from_be_bytes(raw),
        }
    })
}

fn read_i16_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<i16>> {
    read_section(bytes, offsets, slot, count, 2, |bytes, offset| {
        let raw = [bytes[offset], bytes[offset + 1]];
        match endianness {
            Endianness::Little => i16::from_le_bytes(raw),
            Endianness::Big => i16::from_be_bytes(raw),
        }
    })
}

fn read_f32_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    endianness: Endianness,
) -> Result<Vec<f32>> {
    read_section(bytes, offsets, slot, count, 4, |bytes, offset| {
        let raw = [
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ];
        match endianness {
            Endianness::Little => f32::from_le_bytes(raw),
            Endianness::Big => f32::from_be_bytes(raw),
        }
    })
}

fn read_u8_section(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
) -> Result<Vec<u8>> {
    read_section(bytes, offsets, slot, count, 1, |bytes, offset| {
        bytes[offset]
    })
}

fn read_section<T>(
    bytes: &[u8],
    offsets: &Moc3SectionOffsets,
    slot: usize,
    count: usize,
    element_size: usize,
    read: impl Fn(&[u8], usize) -> T,
) -> Result<Vec<T>> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let offset = offsets.section_offset(slot).ok_or_else(|| {
        invalid_art_meshes(format!("section slot {slot} is outside offset table"))
    })?;
    if offset == 0 {
        return Err(invalid_art_meshes(format!(
            "section slot {slot} has no offset"
        )));
    }

    let offset = usize::try_from(offset)
        .map_err(|_| invalid_art_meshes(format!("section slot {slot} offset is too large")))?;
    let byte_len = count
        .checked_mul(element_size)
        .ok_or_else(|| invalid_art_meshes(format!("section slot {slot} size overflows")))?;
    if bytes.len().saturating_sub(offset) < byte_len {
        return Err(invalid_art_meshes(format!(
            "section slot {slot} is incomplete"
        )));
    }

    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        values.push(read(bytes, offset + index * element_size));
    }

    Ok(values)
}

fn validate_mesh_ranges(
    index: usize,
    mesh: Moc3ArtMeshInfo,
    uv_count: usize,
    position_index_count: usize,
    drawable_mask_count: usize,
) -> Result<()> {
    let uv_len = nonnegative_range_len(mesh.vertex_count, 2, "vertex count")?;
    validate_range(mesh.uv_begin_index, uv_len, uv_count, index, "uv")?;

    let position_len = nonnegative_range_len(mesh.position_index_count, 1, "position index count")?;
    validate_range(
        mesh.position_index_begin_index,
        position_len,
        position_index_count,
        index,
        "position index",
    )?;

    let mask_len = nonnegative_range_len(mesh.mask_count, 1, "mask count")?;
    validate_range(
        mesh.mask_begin_index,
        mask_len,
        drawable_mask_count,
        index,
        "mask",
    )
}

fn nonnegative_range_len(value: i32, scale: usize, name: &'static str) -> Result<usize> {
    if value < 0 {
        return Err(invalid_art_meshes(format!("{name} is negative")));
    }

    usize::try_from(value)
        .ok()
        .and_then(|value| value.checked_mul(scale))
        .ok_or_else(|| invalid_art_meshes(format!("{name} range size overflows")))
}

fn validate_range(
    begin: i32,
    len: usize,
    source_len: usize,
    mesh_index: usize,
    name: &'static str,
) -> Result<()> {
    if begin < 0 {
        return Err(invalid_art_meshes(format!(
            "art mesh {mesh_index} {name} begin index is negative"
        )));
    }

    let begin = usize::try_from(begin).map_err(|_| {
        invalid_art_meshes(format!("art mesh {mesh_index} {name} begin is too large"))
    })?;
    let end = begin.checked_add(len).ok_or_else(|| {
        invalid_art_meshes(format!("art mesh {mesh_index} {name} range overflows"))
    })?;

    if end > source_len {
        return Err(invalid_art_meshes(format!(
            "art mesh {mesh_index} {name} range is outside section"
        )));
    }

    Ok(())
}

fn to_usize(value: u32, name: &'static str) -> Result<usize> {
    usize::try_from(value).map_err(|_| invalid_art_meshes(format!("{name} is too large")))
}

fn invalid_art_meshes(message: impl Into<String>) -> Error {
    Error::InvalidMoc3 {
        message: message.into(),
    }
}
