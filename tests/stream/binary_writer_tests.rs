use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    BigEndian,
    BinaryWriter,
    ByteOrder,
    LittleEndian,
};

#[test]
fn test_binary_writer_writes_all_big_endian_methods() {
    let mut writer = BinaryWriter::<_, BigEndian>::new(Vec::new());

    assert_eq!(ByteOrder::BigEndian, writer.byte_order());
    writer
        .write_bytes([0xaa, 0xbb])
        .expect("bytes should be written");
    writer.write_u8(0x12).expect("u8 should be written");
    writer.write_i8(-2).expect("i8 should be written");
    writer.write_u16(0x1234).expect("u16 should be written");
    writer
        .write_u32(0x1234_5678)
        .expect("u32 should be written");
    writer
        .write_u64(0x0123_4567_89ab_cdef)
        .expect("u64 should be written");
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be written");
    writer.write_i16(-0x1234).expect("i16 should be written");
    writer
        .write_i32(-0x0123_4567)
        .expect("i32 should be written");
    writer
        .write_i64(-0x0123_4567_89ab_cdef)
        .expect("i64 should be written");
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("i128 should be written");
    writer.write_f32(12.5).expect("f32 should be written");
    writer.write_f64(-25.25).expect("f64 should be written");
    writer
        .write_utf8_string_u16("hi")
        .expect("u16 string should be written");
    writer
        .write_utf8_string_u32("ok")
        .expect("u32 string should be written");

    assert!(!writer.into_inner().is_empty());
}

#[test]
fn test_binary_writer_writes_little_endian_and_exposes_accessors() {
    let mut writer = BinaryWriter::<_, LittleEndian>::new(Cursor::new(Vec::new()));

    assert_eq!(ByteOrder::LittleEndian, writer.byte_order());
    assert_eq!(0, writer.get_ref().position());
    writer.get_mut().set_position(0);
    writer.write_u16(0x1234).expect("u16 should be written");
    assert_eq!(vec![0x34, 0x12], writer.into_inner().into_inner());
}

#[test]
fn test_binary_writer_reports_length_errors() {
    let mut writer = BinaryWriter::<_, BigEndian>::new(Vec::new());
    let value = "x".repeat(usize::from(u16::MAX) + 1);

    assert_eq!(
        ErrorKind::InvalidInput,
        writer
            .write_utf8_string_u16(&value)
            .expect_err("oversized u16 string should fail")
            .kind()
    );
}
