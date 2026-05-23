use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    ZigZagReadExt,
    ZigZagWriteExt,
};

#[test]
fn test_zig_zag_read_ext_reads_all_methods() {
    let mut bytes = Vec::new();
    bytes.write_zig_zag_i8(i8::MIN).expect("i8 should be encoded");
    bytes.write_zig_zag_i16(-300).expect("i16 should be encoded");
    bytes.write_zig_zag_i32(-0x1f600).expect("i32 should be encoded");
    bytes.write_zig_zag_i64(i64::MIN).expect("i64 should be encoded");
    bytes.write_zig_zag_i128(i128::MIN).expect("i128 should be encoded");
    bytes.write_zig_zag_isize(isize::MIN).expect("isize should be encoded");

    let mut input = Cursor::new(bytes.clone());
    assert_eq!(i8::MIN, input.read_zig_zag_i8().expect("i8 should be read"));
    assert_eq!(-300, input.read_zig_zag_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, input.read_zig_zag_i32().expect("i32 should be read"));
    assert_eq!(i64::MIN, input.read_zig_zag_i64().expect("i64 should be read"));
    assert_eq!(i128::MIN, input.read_zig_zag_i128().expect("i128 should be read"));
    assert_eq!(isize::MIN, input.read_zig_zag_isize().expect("isize should be read"));

    let mut input = Cursor::new(bytes);
    assert_eq!(
        i8::MIN,
        input.read_zig_zag_i8_strict().expect("strict i8 should be read")
    );
    assert_eq!(
        -300,
        input.read_zig_zag_i16_strict().expect("strict i16 should be read")
    );
    assert_eq!(
        -0x1f600,
        input.read_zig_zag_i32_strict().expect("strict i32 should be read")
    );
    assert_eq!(
        i64::MIN,
        input.read_zig_zag_i64_strict().expect("strict i64 should be read")
    );
    assert_eq!(
        i128::MIN,
        input.read_zig_zag_i128_strict().expect("strict i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        input.read_zig_zag_isize_strict().expect("strict isize should be read")
    );
}

#[test]
fn test_zig_zag_read_ext_reports_invalid_data_and_eof() {
    let mut input = Cursor::new([0x80, 0x00]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_zig_zag_i16_strict()
            .expect_err("non-canonical value should fail")
            .kind()
    );

    let mut input = Cursor::new([0x80, 0x80]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_zig_zag_i8()
            .expect_err("unterminated max-width i8 value should fail")
            .kind()
    );

    let mut input = Cursor::new([0x80]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_zig_zag_i64()
            .expect_err("truncated value should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x80, 0x80, 0x80]);
    assert_eq!(
        ErrorKind::InvalidData,
        input
            .read_zig_zag_i16()
            .expect_err("unterminated max-width value should fail")
            .kind()
    );
}
