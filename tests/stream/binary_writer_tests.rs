/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::Write;

use qubit_io::{
    BinaryWriter,
    ByteOrder,
};

#[test]
fn test_binary_writer_writes_values_with_byte_order_switch() {
    let mut writer = BinaryWriter::new(Vec::new(), ByteOrder::BigEndian);
    let mut expected = Vec::new();

    assert_eq!(ByteOrder::BigEndian, writer.byte_order());
    writer.write_u8(0x12).expect("u8 should be written");
    expected.push(0x12);
    writer.write_i8(-2).expect("i8 should be written");
    expected.push((-2i8) as u8);
    writer
        .write_u16(0x1234)
        .expect("big endian u16 should be written");
    expected.extend_from_slice(&0x1234u16.to_be_bytes());
    writer
        .write_i16(-0x1234)
        .expect("big endian i16 should be written");
    expected.extend_from_slice(&(-0x1234i16).to_be_bytes());
    writer
        .write_u32(0x1234_5678)
        .expect("big endian u32 should be written");
    expected.extend_from_slice(&0x1234_5678u32.to_be_bytes());
    writer
        .write_i32(-0x0123_4567)
        .expect("big endian i32 should be written");
    expected.extend_from_slice(&(-0x0123_4567i32).to_be_bytes());
    writer
        .write_u64(0x0123_4567_89ab_cdef)
        .expect("big endian u64 should be written");
    expected.extend_from_slice(&0x0123_4567_89ab_cdefu64.to_be_bytes());
    writer
        .write_i64(-0x0123_4567_89ab_cdef)
        .expect("big endian i64 should be written");
    expected.extend_from_slice(&(-0x0123_4567_89ab_cdefi64).to_be_bytes());
    writer
        .write_utf8_string_u16("be16")
        .expect("big endian u16 string should be written");
    expected.extend_from_slice(&4u16.to_be_bytes());
    expected.extend_from_slice(b"be16");
    writer
        .write_utf8_string_u32("be32")
        .expect("big endian u32 string should be written");
    expected.extend_from_slice(&4u32.to_be_bytes());
    expected.extend_from_slice(b"be32");

    writer.set_byte_order(ByteOrder::LittleEndian);
    assert_eq!(ByteOrder::LittleEndian, writer.byte_order());
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("little endian u128 should be written");
    expected.extend_from_slice(&0x0123_4567_89ab_cdef_fedc_ba98_7654_3210u128.to_le_bytes());
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("little endian i128 should be written");
    expected.extend_from_slice(&(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210i128).to_le_bytes());
    writer
        .write_f32(12.5)
        .expect("little endian f32 should be written");
    expected.extend_from_slice(&12.5f32.to_le_bytes());
    writer
        .write_f64(-25.25)
        .expect("little endian f64 should be written");
    expected.extend_from_slice(&(-25.25f64).to_le_bytes());
    writer
        .write_utf8_string_u16("hello")
        .expect("u16 length-prefixed string should be written");
    expected.extend_from_slice(&5u16.to_le_bytes());
    expected.extend_from_slice(b"hello");
    writer
        .write_utf8_string_u32("world")
        .expect("u32 length-prefixed string should be written");
    expected.extend_from_slice(&5u32.to_le_bytes());
    expected.extend_from_slice(b"world");
    writer.get_mut().extend_from_slice(&[0xaa]);
    expected.push(0xaa);

    assert_eq!(expected.len(), writer.get_ref().len());
    assert_eq!(expected, writer.into_inner());
}

#[test]
fn test_binary_writer_delegates_raw_write_and_flush() {
    let mut writer = BinaryWriter::new(Vec::new(), ByteOrder::BigEndian);

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    assert_eq!(vec![0x01, 0x02], writer.into_inner());
}

#[test]
fn test_binary_writer_covers_opposite_scalar_byte_order_branches() {
    let mut writer = BinaryWriter::new(Vec::new(), ByteOrder::LittleEndian);

    writer
        .write_u16(0x1234)
        .expect("little endian u16 should be written");
    writer
        .write_i16(-0x1234)
        .expect("little endian i16 should be written");
    writer
        .write_u32(0x1234_5678)
        .expect("little endian u32 should be written");
    writer
        .write_i32(-0x0123_4567)
        .expect("little endian i32 should be written");
    writer
        .write_u64(0x0123_4567_89ab_cdef)
        .expect("little endian u64 should be written");
    writer
        .write_i64(-0x0123_4567_89ab_cdef)
        .expect("little endian i64 should be written");

    writer.set_byte_order(ByteOrder::BigEndian);
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("big endian u128 should be written");
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("big endian i128 should be written");
    writer
        .write_f32(12.5)
        .expect("big endian f32 should be written");
    writer
        .write_f64(-25.25)
        .expect("big endian f64 should be written");

    assert!(!writer.into_inner().is_empty());
}

#[test]
fn test_binary_writer_forwards_seek() {
    use std::io::{
        Cursor,
        Seek,
        SeekFrom,
    };

    let mut writer = BinaryWriter::new(Cursor::new(Vec::new()), ByteOrder::BigEndian);

    writer
        .write_all(b"abc")
        .expect("initial write should succeed");
    writer
        .seek(SeekFrom::Start(1))
        .expect("seek should be forwarded");
    writer.write_all(b"z").expect("patch write should succeed");

    assert_eq!(b"azc", writer.into_inner().into_inner().as_slice());
}
