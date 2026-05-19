/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    Cursor,
    Read,
    Seek,
    SeekFrom,
    Write,
};

use qubit_io::{
    BinaryReader,
    BinaryWriter,
    ByteOrder,
};

#[test]
fn test_binary_reader_writer_round_trips_values_with_byte_order_switch() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = BinaryWriter::new(cursor, ByteOrder::BigEndian);

    assert_eq!(ByteOrder::BigEndian, writer.byte_order());
    writer.write_u8(0x12).expect("u8 should be written");
    writer.write_i8(-2).expect("i8 should be written");
    writer
        .write_u16(0x1234)
        .expect("big endian u16 should be written");
    writer
        .write_i16(-0x1234)
        .expect("big endian i16 should be written");
    writer
        .write_u32(0x1234_5678)
        .expect("big endian u32 should be written");
    writer
        .write_i32(-0x0123_4567)
        .expect("big endian i32 should be written");
    writer
        .write_u64(0x0123_4567_89ab_cdef)
        .expect("big endian u64 should be written");
    writer
        .write_i64(-0x0123_4567_89ab_cdef)
        .expect("big endian i64 should be written");
    writer
        .write_utf8_string_u16("be16")
        .expect("big endian u16 string should be written");
    writer
        .write_utf8_string_u32("be32")
        .expect("big endian u32 string should be written");

    writer.set_byte_order(ByteOrder::LittleEndian);
    assert_eq!(ByteOrder::LittleEndian, writer.byte_order());
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("little endian u128 should be written");
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("little endian i128 should be written");
    writer
        .write_f32(12.5)
        .expect("little endian f32 should be written");
    writer
        .write_f64(-25.25)
        .expect("little endian f64 should be written");
    writer
        .write_utf8_string_u16("hello")
        .expect("u16 length-prefixed string should be written");
    writer
        .write_utf8_string_u32("world")
        .expect("u32 length-prefixed string should be written");
    writer
        .get_mut()
        .write_all(&[0xaa])
        .expect("inner writer should remain mutable");

    let mut cursor = writer.into_inner();
    cursor
        .seek(SeekFrom::Start(0))
        .expect("cursor should rewind for reading");
    let mut reader = BinaryReader::new(cursor, ByteOrder::BigEndian);

    assert_eq!(ByteOrder::BigEndian, reader.byte_order());
    assert_eq!(0x12, reader.read_u8().expect("u8 should be read"));
    assert_eq!(-2, reader.read_i8().expect("i8 should be read"));
    assert_eq!(
        0x1234,
        reader.read_u16().expect("big endian u16 should be read")
    );
    assert_eq!(
        -0x1234,
        reader.read_i16().expect("big endian i16 should be read")
    );
    assert_eq!(
        0x1234_5678,
        reader.read_u32().expect("big endian u32 should be read")
    );
    assert_eq!(
        -0x0123_4567,
        reader.read_i32().expect("big endian i32 should be read")
    );
    assert_eq!(
        0x0123_4567_89ab_cdef,
        reader.read_u64().expect("big endian u64 should be read")
    );
    assert_eq!(
        -0x0123_4567_89ab_cdef,
        reader.read_i64().expect("big endian i64 should be read")
    );
    assert_eq!(
        "be16",
        reader
            .read_utf8_string_u16(8)
            .expect("big endian u16 string should be read")
    );
    assert_eq!(
        "be32",
        reader
            .read_utf8_string_u32(8)
            .expect("big endian u32 string should be read")
    );

    reader.set_byte_order(ByteOrder::LittleEndian);
    assert_eq!(ByteOrder::LittleEndian, reader.byte_order());
    assert_eq!(
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader
            .read_u128()
            .expect("little endian u128 should be read")
    );
    assert_eq!(
        -0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader
            .read_i128()
            .expect("little endian i128 should be read")
    );
    assert_eq!(
        12.5,
        reader.read_f32().expect("little endian f32 should be read")
    );
    assert_eq!(
        -25.25,
        reader.read_f64().expect("little endian f64 should be read")
    );
    assert_eq!(
        "hello",
        reader
            .read_utf8_string_u16(16)
            .expect("u16 length-prefixed string should be read")
    );
    assert_eq!(
        "world",
        reader
            .read_utf8_string_u32(16)
            .expect("u32 length-prefixed string should be read")
    );

    let position = reader.get_ref().position();
    reader
        .get_mut()
        .seek(SeekFrom::Start(position))
        .expect("inner reader should remain mutable");
    assert_eq!(0xaa, reader.read_u8().expect("tail byte should be read"));
    let cursor = reader.into_inner();
    assert_eq!(cursor.get_ref().len() as u64, cursor.position());
}

#[test]
fn test_binary_reader_writer_delegate_raw_io_and_accessors() {
    let mut writer = BinaryWriter::new(Vec::new(), ByteOrder::BigEndian);

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    let mut reader = BinaryReader::new(Cursor::new(writer.into_inner()), ByteOrder::BigEndian);
    let mut bytes = [0; 2];
    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}

#[test]
fn test_binary_reader_writer_use_big_endian_string_prefixes() {
    let mut writer = BinaryWriter::new(Vec::new(), ByteOrder::BigEndian);

    writer
        .write_utf8_string_u16("be16")
        .expect("big endian u16 string should be written");
    writer
        .write_utf8_string_u32("be32")
        .expect("big endian u32 string should be written");
    writer.set_byte_order(ByteOrder::LittleEndian);
    writer
        .write_utf8_string_u16("le16")
        .expect("little endian u16 string should be written");
    writer
        .write_utf8_string_u32("le32")
        .expect("little endian u32 string should be written");

    let mut reader = BinaryReader::new(Cursor::new(writer.into_inner()), ByteOrder::BigEndian);

    assert_eq!(
        "be16",
        reader
            .read_utf8_string_u16(8)
            .expect("big endian u16 string should be read")
    );
    assert_eq!(
        "be32",
        reader
            .read_utf8_string_u32(8)
            .expect("big endian u32 string should be read")
    );
    reader.set_byte_order(ByteOrder::LittleEndian);
    assert_eq!(
        "le16",
        reader
            .read_utf8_string_u16(8)
            .expect("little endian u16 string should be read")
    );
    assert_eq!(
        "le32",
        reader
            .read_utf8_string_u32(8)
            .expect("little endian u32 string should be read")
    );
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
