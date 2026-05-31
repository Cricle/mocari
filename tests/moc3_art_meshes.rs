use rusty_live2d::{
    Error,
    moc3::{Moc3ArtMeshInfo, Moc3ArtMeshes},
};

#[test]
fn parses_moc3_art_mesh_render_sections() {
    let bytes = moc3_with_art_meshes();

    let art_meshes = Moc3ArtMeshes::parse(&bytes).unwrap();

    assert_eq!(art_meshes.meshes().len(), 2);
    assert_eq!(
        art_meshes.meshes()[0],
        Moc3ArtMeshInfo::new(1, 0b0000_0011, 6, 0, 0, 4, 0, 1)
    );
    assert_eq!(
        art_meshes.meshes()[1],
        Moc3ArtMeshInfo::new(0, 0b0000_0100, 3, 8, 6, 2, 1, 1)
    );
    assert_eq!(
        art_meshes.art_mesh_uvs(0).unwrap(),
        &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        art_meshes.art_mesh_position_indices(0).unwrap(),
        &[0, 1, 2, 0, 2, 3]
    );
    assert_eq!(art_meshes.art_mesh_masks(0).unwrap(), &[1]);
    assert_eq!(
        art_meshes.art_mesh_uvs(1).unwrap(),
        &[0.25, 0.25, 0.75, 0.75]
    );
    assert_eq!(art_meshes.art_mesh_position_indices(1).unwrap(), &[0, 1, 0]);
    assert_eq!(art_meshes.art_mesh_masks(1).unwrap(), &[0]);
}

#[test]
fn rejects_incomplete_moc3_art_mesh_section() {
    let mut bytes = moc3_with_art_meshes();
    bytes.truncate(0xa90);

    let error = Moc3ArtMeshes::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn moc3_with_art_meshes() -> Vec<u8> {
    let mut bytes = vec![0; 0xc00];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;

    write_u32(&mut bytes, 0x40, 0x7c0);
    write_u32(&mut bytes, 0x44, 0x840);

    write_section_offset(&mut bytes, 41, 0x880);
    write_section_offset(&mut bytes, 42, 0x8c0);
    write_section_offset(&mut bytes, 43, 0x900);
    write_section_offset(&mut bytes, 44, 0x940);
    write_section_offset(&mut bytes, 45, 0x980);
    write_section_offset(&mut bytes, 46, 0x9c0);
    write_section_offset(&mut bytes, 47, 0xa00);
    write_section_offset(&mut bytes, 48, 0xa40);
    write_section_offset(&mut bytes, 78, 0xa80);
    write_section_offset(&mut bytes, 79, 0xb40);
    write_section_offset(&mut bytes, 80, 0xb80);

    write_u32(&mut bytes, 0x7d0, 2);
    write_u32(&mut bytes, 0x7fc, 12);
    write_u32(&mut bytes, 0x800, 9);
    write_u32(&mut bytes, 0x804, 2);

    write_i32_array(&mut bytes, 0x880, &[1, 0]);
    bytes[0x8c0..0x8c2].copy_from_slice(&[0b0000_0011, 0b0000_0100]);
    write_i32_array(&mut bytes, 0x900, &[4, 2]);
    write_i32_array(&mut bytes, 0x940, &[0, 8]);
    write_i32_array(&mut bytes, 0x980, &[0, 6]);
    write_i32_array(&mut bytes, 0x9c0, &[6, 3]);
    write_i32_array(&mut bytes, 0xa00, &[0, 1]);
    write_i32_array(&mut bytes, 0xa40, &[1, 1]);
    write_f32_array(
        &mut bytes,
        0xa80,
        &[
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.25, 0.25, 0.75, 0.75,
        ],
    );
    write_i16_array(&mut bytes, 0xb40, &[0, 1, 2, 0, 2, 3, 0, 1, 0]);
    write_i32_array(&mut bytes, 0xb80, &[1, 0]);

    bytes
}

fn write_section_offset(bytes: &mut [u8], slot: usize, offset: u32) {
    write_u32(bytes, 0x40 + slot * 4, offset);
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32_array(bytes: &mut [u8], offset: usize, values: &[i32]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_i16_array(bytes: &mut [u8], offset: usize, values: &[i16]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 2..offset + index * 2 + 2].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_f32_array(bytes: &mut [u8], offset: usize, values: &[f32]) {
    for (index, value) in values.iter().enumerate() {
        bytes[offset + index * 4..offset + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}
