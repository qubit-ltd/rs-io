use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    ByteOrder,
    StringReadExt,
};

#[test]
fn test_string_read_ext_reads_all_length_prefix_kinds() {
    let mut input = Cursor::new(b"raw".to_vec());
    assert_eq!(
        "raw",
        input
            .read_utf8_payload(3, 8)
            .expect("known-length payload should be read")
    );

    let mut input = Cursor::new(vec![5, b'h', b'e', b'l', b'l', b'o']);
    assert_eq!(
        "hello",
        input.read_utf8_string_uleb(8).expect("ULEB string should be read")
    );

    let mut input = Cursor::new(vec![5, b'h', b'e', b'l', b'l', b'o']);
    assert_eq!(
        "hello",
        input
            .read_utf8_string_uleb_strict(8)
            .expect("strict ULEB string should be read")
    );

    let mut input = Cursor::new(vec![0, 2, b'h', b'i']);
    assert_eq!(
        "hi",
        input.read_utf8_string_u16_be(8).expect("u16 BE string should be read")
    );

    let mut input = Cursor::new(vec![0, 2, b'h', b'i']);
    assert_eq!(
        "hi",
        input
            .read_utf8_string_u16(ByteOrder::BigEndian, 8)
            .expect("runtime u16 BE string should be read")
    );

    let mut input = Cursor::new(vec![2, 0, b'h', b'i']);
    assert_eq!(
        "hi",
        input.read_utf8_string_u16_le(8).expect("u16 LE string should be read")
    );

    let mut input = Cursor::new(vec![2, 0, b'h', b'i']);
    assert_eq!(
        "hi",
        input
            .read_utf8_string_u16(ByteOrder::LittleEndian, 8)
            .expect("runtime u16 LE string should be read")
    );

    let mut input = Cursor::new(vec![0, 0, 0, 2, b'o', b'k']);
    assert_eq!(
        "ok",
        input.read_utf8_string_u32_be(8).expect("u32 BE string should be read")
    );

    let mut input = Cursor::new(vec![0, 0, 0, 2, b'o', b'k']);
    assert_eq!(
        "ok",
        input
            .read_utf8_string_u32(ByteOrder::BigEndian, 8)
            .expect("runtime u32 BE string should be read")
    );

    let mut input = Cursor::new(vec![2, 0, 0, 0, b'o', b'k']);
    assert_eq!(
        "ok",
        input.read_utf8_string_u32_le(8).expect("u32 LE string should be read")
    );

    let mut input = Cursor::new(vec![2, 0, 0, 0, b'o', b'k']);
    assert_eq!(
        "ok",
        input
            .read_utf8_string_u32(ByteOrder::LittleEndian, 8)
            .expect("runtime u32 LE string should be read")
    );
}

#[test]
fn test_string_read_ext_reports_length_and_utf8_errors() {
    let mut input = Cursor::new(vec![b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_payload(3, 2)
            .expect_err("oversized known-length payload should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![3, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_uleb(2)
            .expect_err("oversized ULEB string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0x80, 0x00, b'a']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_uleb_strict(8)
            .expect_err("non-canonical ULEB length should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0, 2, 0xff, 0xff]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u16_be(8)
            .expect_err("invalid UTF-8 should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![2, 0, 0xff, 0xff]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u16_le(8)
            .expect_err("invalid UTF-8 should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0, 0, 0, 3, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u32_be(2)
            .expect_err("oversized u32 BE string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![3, 0, 0, 0, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u32_le(2)
            .expect_err("oversized u32 LE string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0x00, 0x03, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u16(ByteOrder::BigEndian, 2)
            .expect_err("oversized runtime u16 string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0x03, 0x00, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u16(ByteOrder::LittleEndian, 2)
            .expect_err("oversized runtime little-endian u16 string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0x00, 0x00, 0x00, 0x03, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u32(ByteOrder::BigEndian, 2)
            .expect_err("oversized runtime u32 string should fail")
            .kind()
    );

    let mut input = Cursor::new(vec![0x03, 0x00, 0x00, 0x00, b'a', b'b', b'c']);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_utf8_string_u32(ByteOrder::LittleEndian, 2)
            .expect_err("oversized runtime little-endian u32 string should fail")
            .kind()
    );
}

#[test]
fn test_string_read_ext_returns_payload_read_error() {
    let mut input = Cursor::new(vec![2, b'a']);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_utf8_string_uleb(8)
            .expect_err("truncated payload should fail")
            .kind()
    );

    let mut reader = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_uleb(8)
            .expect_err("length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u16(ByteOrder::BigEndian, 8)
            .expect_err("runtime u16 length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u16(ByteOrder::LittleEndian, 8)
            .expect_err("runtime little-endian u16 length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u16_be(8)
            .expect_err("u16 BE length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u16_le(8)
            .expect_err("u16 LE length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00, 0x00, 0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u32(ByteOrder::BigEndian, 8)
            .expect_err("runtime u32 length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00, 0x00, 0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u32(ByteOrder::LittleEndian, 8)
            .expect_err("runtime little-endian u32 length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00, 0x00, 0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u32_be(8)
            .expect_err("u32 BE length read error should be returned")
            .kind()
    );

    let mut reader = Cursor::new(vec![0x00, 0x00, 0x00]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_utf8_string_u32_le(8)
            .expect_err("u32 LE length read error should be returned")
            .kind()
    );

    let mut input = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        ErrorKind::Other,
        input
            .read_utf8_payload(usize::MAX, usize::MAX)
            .expect_err("payload allocation error should be returned")
            .kind()
    );
}
