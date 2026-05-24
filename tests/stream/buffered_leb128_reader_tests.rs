use std::io::{Cursor, ErrorKind};

use qubit_io::{BufferedLeb128Reader, Leb128WriteExt, NonStrict, Strict};

#[test]
fn test_buffered_leb128_reader_reads_values_across_buffer_boundaries() {
    let mut bytes = Vec::new();
    bytes.write_uleb_u8(u8::MAX).expect("u8 should be encoded");
    bytes.write_uleb_u16(300).expect("u16 should be encoded");
    bytes
        .write_uleb_u32(0x1f600)
        .expect("u32 should be encoded");
    bytes
        .write_uleb_u64(0x0102_0304_0506_0708)
        .expect("u64 should be encoded");
    bytes.write_sleb_i8(i8::MIN).expect("i8 should be encoded");
    bytes.write_sleb_i16(-300).expect("i16 should be encoded");
    bytes
        .write_sleb_i32(-0x1f600)
        .expect("i32 should be encoded");
    bytes
        .write_sleb_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be encoded");

    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new(bytes), 3);

    assert!(!reader.is_strict());
    assert_eq!(u8::MAX, reader.read_u8().expect("u8 should be read"));
    assert_eq!(300, reader.read_u16().expect("u16 should be read"));
    assert_eq!(0x1f600, reader.read_u32().expect("u32 should be read"));
    assert_eq!(
        0x0102_0304_0506_0708,
        reader.read_u64().expect("u64 should be read")
    );
    assert_eq!(i8::MIN, reader.read_i8().expect("i8 should be read"));
    assert_eq!(-300, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, reader.read_i32().expect("i32 should be read"));
    assert_eq!(
        -0x0102_0304_0506_0708,
        reader.read_i64().expect("i64 should be read")
    );
}

#[test]
fn test_buffered_leb128_reader_reports_invalid_and_truncated_values() {
    let mut reader = BufferedLeb128Reader::<_, Strict>::with_capacity(Cursor::new([0x80, 0x00]), 2);
    assert!(reader.is_strict());
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u16()
            .expect_err("non-canonical value should fail")
            .kind()
    );

    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new([0x80]), 2);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_u64()
            .expect_err("truncated value should report EOF")
            .kind()
    );
}

#[test]
fn test_buffered_leb128_reader_consumes_invalid_payload_before_reporting_error() {
    let mut reader =
        BufferedLeb128Reader::<_, Strict>::with_capacity(Cursor::new([0x80, 0x00, 0x01]), 2);

    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u16()
            .expect_err("non-canonical value should fail")
            .kind()
    );
    assert_eq!(
        1,
        reader.read_u8().expect("next value should remain readable")
    );

    let mut reader =
        BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new([0x80, 0x02, 0x01]), 2);
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u8()
            .expect_err("out-of-range u8 encoding should fail")
            .kind()
    );
    assert_eq!(
        1,
        reader.read_u8().expect("next value should remain readable")
    );
}
