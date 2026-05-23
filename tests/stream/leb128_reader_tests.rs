use std::io::{Cursor, ErrorKind};

use qubit_io::{Leb128Reader, Leb128Writer, NonStrict, Strict};

#[test]
fn test_leb128_reader_reads_all_methods() {
    let mut writer = Leb128Writer::new(Vec::new());
    writer.write_u8(u8::MAX).expect("u8 should be written");
    writer.write_u16(300).expect("u16 should be written");
    writer.write_u32(0x1f600).expect("u32 should be written");
    writer
        .write_u64(0x0102_0304_0506_0708)
        .expect("u64 should be written");
    writer
        .write_u128(0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("u128 should be written");
    writer
        .write_usize(usize::MAX)
        .expect("usize should be written");
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer
        .write_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be written");
    writer
        .write_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize should be written");

    let mut reader = Leb128Reader::<_, NonStrict>::new(Cursor::new(writer.into_inner()));
    assert!(!reader.is_strict());
    assert_eq!(u8::MAX, reader.read_u8().expect("u8 should be read"));
    assert_eq!(300, reader.read_u16().expect("u16 should be read"));
    assert_eq!(0x1f600, reader.read_u32().expect("u32 should be read"));
    assert_eq!(
        0x0102_0304_0506_0708,
        reader.read_u64().expect("u64 should be read")
    );
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        reader.read_u128().expect("u128 should be read")
    );
    assert_eq!(
        usize::MAX,
        reader.read_usize().expect("usize should be read")
    );
    assert_eq!(i8::MIN, reader.read_i8().expect("i8 should be read"));
    assert_eq!(-300, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, reader.read_i32().expect("i32 should be read"));
    assert_eq!(
        -0x0102_0304_0506_0708,
        reader.read_i64().expect("i64 should be read")
    );
    assert_eq!(
        -0x0102_0304_0506_0708_1112_1314_1516_1718,
        reader.read_i128().expect("i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        reader.read_isize().expect("isize should be read")
    );
}

#[test]
fn test_leb128_reader_exposes_accessors_and_reports_errors() {
    let mut reader = Leb128Reader::<_, Strict>::new(Cursor::new([0x80, 0x00]));
    assert!(reader.is_strict());
    assert_eq!(0, reader.get_ref().position());
    reader.get_mut().set_position(0);
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u16()
            .expect_err("non-canonical value should fail")
            .kind()
    );
    assert_eq!(2, reader.into_inner().position());

    let mut reader = Leb128Reader::<_, NonStrict>::new(Cursor::new([0x80]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_u64()
            .expect_err("truncated value should report EOF")
            .kind()
    );

    let mut reader = Leb128Reader::<_, NonStrict>::new(Cursor::new([0x80, 0x80, 0x80]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u16()
            .expect_err("unterminated max-width value should fail")
            .kind()
    );
}

#[test]
fn test_leb128_reader_read_utf8_string_reads_length_prefixed_payload() {
    let bytes = vec![3, b'h', 0xC3, 0xA9];
    let mut reader =
        qubit_io::Leb128Reader::<_, qubit_io::NonStrict>::new(std::io::Cursor::new(bytes));

    let text = reader
        .read_utf8_string(3)
        .expect("reading a length-prefixed UTF-8 string should succeed");

    assert_eq!(text, "hé");
}

#[test]
fn test_leb128_reader_read_and_seek_delegate_to_inner_reader() {
    let mut reader =
        qubit_io::Leb128Reader::<_, qubit_io::NonStrict>::new(std::io::Cursor::new(vec![
            1, 2, 3, 4,
        ]));

    std::io::Seek::seek(&mut reader, std::io::SeekFrom::Start(1))
        .expect("seeking through Leb128Reader should succeed");
    let mut bytes = [0_u8; 2];
    std::io::Read::read_exact(&mut reader, &mut bytes)
        .expect("reading through Leb128Reader should succeed");

    assert_eq!(bytes, [2, 3]);
}
