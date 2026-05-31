use rusty_live2d::{Error, moc3::Moc3CountInfo};

#[test]
fn parses_basic_moc3_count_info() {
    let bytes = moc3_with_counts(&[
        15, 49, 41, 8, 83, 29, 15, 242, 24, 414, 41472, 35, 29, 29, 85, 4006, 7624, 63, 21, 23, 0,
        0, 0,
    ]);

    let counts = Moc3CountInfo::parse(&bytes).unwrap();

    assert_eq!(counts.parts(), 15);
    assert_eq!(counts.deformers(), 49);
    assert_eq!(counts.art_meshes(), 83);
    assert_eq!(counts.parameters(), 29);
    assert_eq!(counts.uvs(), 4006);
    assert_eq!(counts.position_indices(), 7624);
}

#[test]
fn rejects_incomplete_moc3_count_info() {
    let bytes = moc3_with_count_fixture(&[15, 49], 0x890, 0x880, 0x840);
    let error = Moc3CountInfo::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

fn moc3_with_counts(counts: &[u32]) -> Vec<u8> {
    moc3_with_count_fixture(counts, 0x880, 0x7c0, 0x840)
}

fn moc3_with_count_fixture(
    counts: &[u32],
    len: usize,
    count_offset: u32,
    canvas_offset: u32,
) -> Vec<u8> {
    let mut bytes = vec![0; len];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = 1;
    bytes[0x40..0x44].copy_from_slice(&count_offset.to_le_bytes());
    bytes[0x44..0x48].copy_from_slice(&canvas_offset.to_le_bytes());

    let mut cursor = count_offset as usize;
    for count in counts {
        bytes[cursor..cursor + 4].copy_from_slice(&count.to_le_bytes());
        cursor += 4;
    }

    bytes
}
