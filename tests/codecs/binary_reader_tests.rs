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
};

use qubit_io::{
    BinaryReader,
    ByteOrder,
};

#[test]
fn test_binary_reader_reads_values_with_byte_order_switch() {
    let mut data = Vec::new();
    data.push(0x12);
    data.push((-2i8) as u8);
    data.extend_from_slice(&0x1234u16.to_be_bytes());
    data.extend_from_slice(&(-0x1234i16).to_be_bytes());
    data.extend_from_slice(&0x1234_5678u32.to_be_bytes());
    data.extend_from_slice(&(-0x0123_4567i32).to_be_bytes());
    data.extend_from_slice(&0x0123_4567_89ab_cdefu64.to_be_bytes());
    data.extend_from_slice(&(-0x0123_4567_89ab_cdefi64).to_be_bytes());
    data.extend_from_slice(&4u16.to_be_bytes());
    data.extend_from_slice(b"be16");
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(b"be32");

    data.extend_from_slice(&0x0123_4567_89ab_cdef_fedc_ba98_7654_3210u128.to_le_bytes());
    data.extend_from_slice(&(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210i128).to_le_bytes());
    data.extend_from_slice(&12.5f32.to_le_bytes());
    data.extend_from_slice(&(-25.25f64).to_le_bytes());
    data.extend_from_slice(&5u16.to_le_bytes());
    data.extend_from_slice(b"hello");
    data.extend_from_slice(&5u32.to_le_bytes());
    data.extend_from_slice(b"world");
    data.push(0xaa);

    let cursor = Cursor::new(data);
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
fn test_binary_reader_delegates_raw_read() {
    let mut reader = BinaryReader::new(Cursor::new(vec![0x01, 0x02]), ByteOrder::BigEndian);
    let mut bytes = [0; 2];

    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}
