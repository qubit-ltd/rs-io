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
    std::io::Write::write_all(&mut writer, &[0xaa, 0xbb]).expect("bytes should be written");
    writer.write_u8(0x12).expect("u8 should be written");
    writer.write_i8(-2).expect("i8 should be written");
    writer.write_u16(0x1234).expect("u16 should be written");
    writer.write_u32(0x1234_5678).expect("u32 should be written");
    writer.write_u64(0x0123_4567_89ab_cdef).expect("u64 should be written");
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be written");
    writer.write_i16(-0x1234).expect("i16 should be written");
    writer.write_i32(-0x0123_4567).expect("i32 should be written");
    writer.write_i64(-0x0123_4567_89ab_cdef).expect("i64 should be written");
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
    writer.write_u8(0x12).expect("u8 should be written");
    writer.write_i8(-2).expect("i8 should be written");
    writer.write_u16(0x1234).expect("u16 should be written");
    writer.write_u32(0x1234_5678).expect("u32 should be written");
    writer.write_u64(0x0123_4567_89ab_cdef).expect("u64 should be written");
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be written");
    writer.write_i16(-0x1234).expect("i16 should be written");
    writer.write_i32(-0x0123_4567).expect("i32 should be written");
    writer.write_i64(-0x0123_4567_89ab_cdef).expect("i64 should be written");
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

    let mut expected = Vec::new();
    expected.push(0x12);
    expected.push((-2_i8) as u8);
    expected.extend_from_slice(&0x1234_u16.to_le_bytes());
    expected.extend_from_slice(&0x1234_5678_u32.to_le_bytes());
    expected.extend_from_slice(&0x0123_4567_89ab_cdef_u64.to_le_bytes());
    expected.extend_from_slice(&0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_u128.to_le_bytes());
    expected.extend_from_slice(&(-0x1234_i16).to_le_bytes());
    expected.extend_from_slice(&(-0x0123_4567_i32).to_le_bytes());
    expected.extend_from_slice(&(-0x0123_4567_89ab_cdef_i64).to_le_bytes());
    expected.extend_from_slice(&(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_i128).to_le_bytes());
    expected.extend_from_slice(&12.5_f32.to_bits().to_le_bytes());
    expected.extend_from_slice(&(-25.25_f64).to_bits().to_le_bytes());
    expected.extend_from_slice(&2_u16.to_le_bytes());
    expected.extend_from_slice(b"hi");
    expected.extend_from_slice(&2_u32.to_le_bytes());
    expected.extend_from_slice(b"ok");
    assert_eq!(expected, writer.into_inner().into_inner());
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

#[test]
fn test_binary_writer_write_and_seek_delegate_to_inner_writer() {
    let mut writer = qubit_io::BinaryWriter::<_, qubit_io::LittleEndian>::new(std::io::Cursor::new(vec![0; 4]));

    std::io::Seek::seek(&mut writer, std::io::SeekFrom::Start(1)).expect("seeking through BinaryWriter should succeed");
    std::io::Write::write_all(&mut writer, b"xy").expect("writing through BinaryWriter should succeed");
    std::io::Write::flush(&mut writer).expect("flushing through BinaryWriter should succeed");

    let cursor = writer.into_inner();
    assert_eq!(cursor.into_inner(), vec![0, b'x', b'y', 0]);
}
