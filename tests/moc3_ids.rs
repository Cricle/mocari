use rusty_live2d::{Error, moc3::Moc3Ids};

#[test]
fn parses_moc3_fixed_width_id_sections() {
    let bytes = moc3_with_ids();

    let ids = Moc3Ids::parse(&bytes).unwrap();

    assert_eq!(ids.parts(), &["PartArmL", "PartArmR"]);
    assert_eq!(ids.art_meshes(), &["ArtMesh00", "ArtMesh01"]);
    assert_eq!(ids.parameters(), &["ParamAngleX", "ParamEyeLOpen"]);
}

#[test]
fn rejects_incomplete_moc3_id_section() {
    let mut bytes = moc3_with_ids();
    bytes.truncate(0x920);

    let error = Moc3Ids::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn moc3_with_ids() -> Vec<u8> {
    let mut bytes = vec![0; 0xa00];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;

    write_u32(&mut bytes, 0x40, 0x7c0);
    write_u32(&mut bytes, 0x44, 0x840);
    write_u32(&mut bytes, 0x4c, 0x880);
    write_u32(&mut bytes, 0xc4, 0x900);
    write_u32(&mut bytes, 0x108, 0x980);

    write_u32(&mut bytes, 0x7c0, 2);
    write_u32(&mut bytes, 0x7d0, 2);
    write_u32(&mut bytes, 0x7d4, 2);

    write_str64(&mut bytes, 0x880, "PartArmL");
    write_str64(&mut bytes, 0x8c0, "PartArmR");
    write_str64(&mut bytes, 0x900, "ArtMesh00");
    write_str64(&mut bytes, 0x940, "ArtMesh01");
    write_str64(&mut bytes, 0x980, "ParamAngleX");
    write_str64(&mut bytes, 0x9c0, "ParamEyeLOpen");

    bytes
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_str64(bytes: &mut [u8], offset: usize, value: &str) {
    bytes[offset..offset + value.len()].copy_from_slice(value.as_bytes());
}
