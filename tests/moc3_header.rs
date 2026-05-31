use rusty_live2d::{
    Error,
    moc3::{Endianness, Moc3Header, Moc3Version},
};

#[test]
fn parses_moc3_header() {
    let header = Moc3Header::parse(&header_bytes(1, 0)).unwrap();

    assert_eq!(header.version(), Moc3Version::V3_0_0);
    assert_eq!(header.endianness(), Endianness::Little);
}

#[test]
fn parses_big_endian_moc3_header_flag() {
    let header = Moc3Header::parse(&header_bytes(5, 1)).unwrap();

    assert_eq!(header.version(), Moc3Version::V5_0_0);
    assert_eq!(header.endianness(), Endianness::Big);
}

#[test]
fn rejects_short_moc3_header() {
    let error = Moc3Header::parse(b"MOC3").unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

#[test]
fn rejects_invalid_moc3_magic() {
    let mut bytes = header_bytes(1, 0);
    bytes[0..4].copy_from_slice(b"NOPE");

    let error = Moc3Header::parse(&bytes).unwrap_err();

    assert!(matches!(error, Error::InvalidMoc3 { .. }));
}

#[test]
fn rejects_unknown_moc3_version() {
    let error = Moc3Header::parse(&header_bytes(99, 0)).unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedVersion {
            format: "moc3",
            version: 99
        }
    ));
}

fn header_bytes(version: u8, big_endian: u8) -> [u8; 64] {
    let mut bytes = [0; 64];
    bytes[0..4].copy_from_slice(b"MOC3");
    bytes[4] = version;
    bytes[5] = big_endian;
    bytes
}
