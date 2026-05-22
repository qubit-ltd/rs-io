use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    Leb128ReadExt,
    Leb128WriteExt,
};

#[test]
fn test_leb128_read_ext_reads_all_unsigned_methods() {
    let mut bytes = Vec::new();
    bytes.write_uleb_u8(u8::MAX).expect("u8 should be encoded");
    bytes.write_uleb_u16(300).expect("u16 should be encoded");
    bytes
        .write_uleb_u32(0x1f600)
        .expect("u32 should be encoded");
    bytes
        .write_uleb_u64(0x0102_0304_0506_0708)
        .expect("u64 should be encoded");
    bytes
        .write_uleb_u128(0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("u128 should be encoded");
    bytes
        .write_uleb_usize(usize::MAX)
        .expect("usize should be encoded");

    let mut input = Cursor::new(bytes.clone());
    assert_eq!(u8::MAX, input.read_uleb_u8().expect("u8 should be read"));
    assert_eq!(300, input.read_uleb_u16().expect("u16 should be read"));
    assert_eq!(0x1f600, input.read_uleb_u32().expect("u32 should be read"));
    assert_eq!(
        0x0102_0304_0506_0708,
        input.read_uleb_u64().expect("u64 should be read")
    );
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        input.read_uleb_u128().expect("u128 should be read")
    );
    assert_eq!(
        usize::MAX,
        input.read_uleb_usize().expect("usize should be read")
    );

    let mut input = Cursor::new(bytes);
    assert_eq!(
        u8::MAX,
        input
            .read_uleb_u8_strict()
            .expect("strict u8 should be read")
    );
    assert_eq!(
        300,
        input
            .read_uleb_u16_strict()
            .expect("strict u16 should be read")
    );
    assert_eq!(
        0x1f600,
        input
            .read_uleb_u32_strict()
            .expect("strict u32 should be read")
    );
    assert_eq!(
        0x0102_0304_0506_0708,
        input
            .read_uleb_u64_strict()
            .expect("strict u64 should be read")
    );
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        input
            .read_uleb_u128_strict()
            .expect("strict u128 should be read")
    );
    assert_eq!(
        usize::MAX,
        input
            .read_uleb_usize_strict()
            .expect("strict usize should be read")
    );
}

#[test]
fn test_leb128_read_ext_reads_all_signed_methods() {
    let mut bytes = Vec::new();
    bytes.write_sleb_i8(i8::MIN).expect("i8 should be encoded");
    bytes.write_sleb_i16(-300).expect("i16 should be encoded");
    bytes
        .write_sleb_i32(-0x1f600)
        .expect("i32 should be encoded");
    bytes
        .write_sleb_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be encoded");
    bytes
        .write_sleb_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be encoded");
    bytes
        .write_sleb_isize(isize::MIN)
        .expect("isize should be encoded");

    let mut input = Cursor::new(bytes.clone());
    assert_eq!(i8::MIN, input.read_sleb_i8().expect("i8 should be read"));
    assert_eq!(-300, input.read_sleb_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, input.read_sleb_i32().expect("i32 should be read"));
    assert_eq!(
        -0x0102_0304_0506_0708,
        input.read_sleb_i64().expect("i64 should be read")
    );
    assert_eq!(
        -0x0102_0304_0506_0708_1112_1314_1516_1718,
        input.read_sleb_i128().expect("i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        input.read_sleb_isize().expect("isize should be read")
    );

    let mut input = Cursor::new(bytes);
    assert_eq!(
        i8::MIN,
        input
            .read_sleb_i8_strict()
            .expect("strict i8 should be read")
    );
    assert_eq!(
        -300,
        input
            .read_sleb_i16_strict()
            .expect("strict i16 should be read")
    );
    assert_eq!(
        -0x1f600,
        input
            .read_sleb_i32_strict()
            .expect("strict i32 should be read")
    );
    assert_eq!(
        -0x0102_0304_0506_0708,
        input
            .read_sleb_i64_strict()
            .expect("strict i64 should be read")
    );
    assert_eq!(
        -0x0102_0304_0506_0708_1112_1314_1516_1718,
        input
            .read_sleb_i128_strict()
            .expect("strict i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        input
            .read_sleb_isize_strict()
            .expect("strict isize should be read")
    );
}

#[test]
fn test_leb128_read_ext_reports_invalid_data_and_eof() {
    let mut input = Cursor::new([0x80, 0x00]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_uleb_u16_strict()
            .expect_err("non-canonical value should fail")
            .kind()
    );

    let mut input = Cursor::new([0x80]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_uleb_u64()
            .expect_err("truncated value should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x80, 0x80, 0x80]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_uleb_u16()
            .expect_err("unterminated max-width value should fail")
            .kind()
    );
}
