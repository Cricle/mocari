use rusty_live2d::{Error, moc3::Moc3SectionOffsets};

#[test]
fn parses_confirmed_moc3_section_offsets() {
    let bytes = moc3_with_offsets(0x7c0, 0x840, 0x900);
    let offsets = Moc3SectionOffsets::parse(&bytes).unwrap();

    assert_eq!(offsets.count_info_offset(), 0x7c0);
    assert_eq!(offsets.canvas_info_offset(), 0x840);
}

#[test]
fn rejects_moc3_without_section_offset_table() {
    let bytes = header_only();
    let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

#[test]
fn rejects_out_of_range_moc3_section_offsets() {
    let bytes = moc3_with_offsets(0x7c0, 0x940, 0x900);
    let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

#[test]
fn rejects_moc3_section_offsets_that_point_into_header_or_table() {
    let bytes = moc3_with_offsets(0, 0x840, 0x900);
    let error = Moc3SectionOffsets::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn header_only() -> [u8; 64] {
    let mut bytes = [0; 64];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;
    bytes
}

fn moc3_with_offsets(count_info_offset: u32, canvas_info_offset: u32, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    bytes[0..64].copy_from_slice(&header_only());
    bytes[0x40..0x44].copy_from_slice(&count_info_offset.to_le_bytes());
    bytes[0x44..0x48].copy_from_slice(&canvas_info_offset.to_le_bytes());
    bytes
}
