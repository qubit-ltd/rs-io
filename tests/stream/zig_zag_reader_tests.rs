use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    NonStrict,
    Strict,
    ZigZagCodec,
    ZigZagReader,
    ZigZagWriter,
};

#[test]
fn test_zig_zag_reader_reads_all_methods() {
    let mut writer = ZigZagWriter::new(Vec::new());
    writer
        .write_i8(0)
        .expect("single-byte i8 should be written");
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer.write_i64(i64::MIN).expect("i64 should be written");
    writer
        .write_i128(i128::MIN)
        .expect("i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize should be written");

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(writer.into_inner()));
    assert!(!reader.is_strict());
    assert_eq!(0, reader.read_i8().expect("single-byte i8 should be read"));
    assert_eq!(i8::MIN, reader.read_i8().expect("i8 should be read"));
    assert_eq!(-300, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, reader.read_i32().expect("i32 should be read"));
    assert_eq!(i64::MIN, reader.read_i64().expect("i64 should be read"));
    assert_eq!(i128::MIN, reader.read_i128().expect("i128 should be read"));
    assert_eq!(
        isize::MIN,
        reader.read_isize().expect("isize should be read")
    );

    let mut writer = ZigZagWriter::new(Vec::new());
    writer.write_i8(0).expect("strict i8 should be written");
    writer
        .write_i16(-300)
        .expect("strict i16 should be written");
    writer
        .write_i32(-0x1f600)
        .expect("strict i32 should be written");
    writer
        .write_i64(i64::MIN)
        .expect("strict i64 should be written");
    writer
        .write_i128(i128::MIN)
        .expect("strict i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("strict isize should be written");

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new(writer.into_inner()));
    assert!(reader.is_strict());
    assert_eq!(0, reader.read_i8().expect("strict i8 should be read"));
    assert_eq!(-300, reader.read_i16().expect("strict i16 should be read"));
    assert_eq!(
        -0x1f600,
        reader.read_i32().expect("strict i32 should be read")
    );
    assert_eq!(
        i64::MIN,
        reader.read_i64().expect("strict i64 should be read")
    );
    assert_eq!(
        i128::MIN,
        reader.read_i128().expect("strict i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        reader.read_isize().expect("strict isize should be read")
    );
}

#[test]
fn test_zig_zag_reader_exposes_accessors_and_reports_errors() {
    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert!(reader.is_strict());
    assert_eq!(0, reader.get_ref().position());
    reader.get_mut().set_position(0);
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_i16()
            .expect_err("non-canonical value should fail")
            .kind()
    );
    assert_eq!(2, reader.into_inner().position());

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_i64()
            .expect_err("truncated value should report EOF")
            .kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80, 0x80, 0x80]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_i16()
            .expect_err("unterminated max-width value should fail")
            .kind()
    );
}

#[test]
fn test_zig_zag_reader_reports_all_instantiated_error_paths() {
    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_i8().expect_err("truncated i8").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_i16().expect_err("truncated i16").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_i32().expect_err("truncated i32").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_i64().expect_err("truncated i64").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_i128().expect_err("truncated i128").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_isize().expect_err("truncated isize").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i8().expect_err("non-canonical i8").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i16().expect_err("non-canonical i16").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i32().expect_err("non-canonical i32").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i64().expect_err("non-canonical i64").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i128().expect_err("non-canonical i128").kind()
    );

    let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_isize().expect_err("non-canonical isize").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<i8, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i8().expect_err("unterminated i8").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i16().expect_err("unterminated i16").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<i32, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i32().expect_err("unterminated i32").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<i64, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i64().expect_err("unterminated i64").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<i128, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_i128().expect_err("unterminated i128").kind()
    );

    let mut reader = ZigZagReader::<_, NonStrict>::new(Cursor::new(
        [0x80u8; ZigZagCodec::<isize, NonStrict>::REQUIRED_MIN_BUFFER_LEN],
    ));
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_isize().expect_err("unterminated isize").kind()
    );
}
