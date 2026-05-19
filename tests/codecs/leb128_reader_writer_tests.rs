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
    ErrorKind,
    Read,
    Write,
};

use qubit_io::{
    Leb128Reader,
    Leb128Writer,
};

#[test]
fn test_leb128_reader_writer_round_trips_boundary_values_and_strings() {
    let mut writer = Leb128Writer::new(Vec::new());

    writer.write_uleb_u8(0).expect("u8 zero should be written");
    writer
        .write_uleb_u8(u8::MAX)
        .expect("u8 max should be written");
    writer
        .write_uleb_u16(u16::MAX)
        .expect("u16 max should be written");
    writer
        .write_uleb_u32(u32::MAX)
        .expect("u32 max should be written");
    writer
        .write_uleb_u64(u64::MAX)
        .expect("u64 max should be written");
    writer
        .write_uleb_u128(u128::MAX)
        .expect("u128 max should be written");
    writer
        .write_uleb_usize(usize::MAX)
        .expect("usize max should be written");
    writer.write_sleb_i8(0).expect("i8 zero should be written");
    writer
        .write_sleb_i8(i8::MIN)
        .expect("i8 min should be written");
    writer
        .write_sleb_i16(i16::MAX)
        .expect("i16 max should be written");
    writer
        .write_sleb_i32(i32::MIN)
        .expect("i32 min should be written");
    writer
        .write_sleb_i64(i64::MAX)
        .expect("i64 max should be written");
    writer
        .write_sleb_i128(i128::MIN)
        .expect("i128 min should be written");
    writer
        .write_sleb_isize(isize::MIN)
        .expect("isize min should be written");
    writer
        .write_utf8_string("hello 世界")
        .expect("string should be written");
    writer
        .write_utf8_string("plain")
        .expect("second string should be written");
    writer.get_mut().push(0x7f);

    assert_eq!(
        0x7f,
        *writer.get_ref().last().expect("tail byte should exist")
    );

    let bytes = writer.into_inner();
    let mut reader = Leb128Reader::new(Cursor::new(bytes));

    assert_eq!(0, reader.read_uleb_u8().expect("u8 zero should be read"));
    assert_eq!(
        u8::MAX,
        reader.read_uleb_u8_strict().expect("u8 max should be read")
    );
    assert_eq!(
        u16::MAX,
        reader
            .read_uleb_u16_strict()
            .expect("u16 max should be read")
    );
    assert_eq!(
        u32::MAX,
        reader
            .read_uleb_u32_strict()
            .expect("u32 max should be read")
    );
    assert_eq!(
        u64::MAX,
        reader
            .read_uleb_u64_strict()
            .expect("u64 max should be read")
    );
    assert_eq!(
        u128::MAX,
        reader
            .read_uleb_u128_strict()
            .expect("u128 max should be read")
    );
    assert_eq!(
        usize::MAX,
        reader
            .read_uleb_usize_strict()
            .expect("usize max should be read")
    );
    assert_eq!(0, reader.read_sleb_i8().expect("i8 zero should be read"));
    assert_eq!(
        i8::MIN,
        reader.read_sleb_i8_strict().expect("i8 min should be read")
    );
    assert_eq!(
        i16::MAX,
        reader
            .read_sleb_i16_strict()
            .expect("i16 max should be read")
    );
    assert_eq!(
        i32::MIN,
        reader
            .read_sleb_i32_strict()
            .expect("i32 min should be read")
    );
    assert_eq!(
        i64::MAX,
        reader
            .read_sleb_i64_strict()
            .expect("i64 max should be read")
    );
    assert_eq!(
        i128::MIN,
        reader
            .read_sleb_i128_strict()
            .expect("i128 min should be read")
    );
    assert_eq!(
        isize::MIN,
        reader
            .read_sleb_isize_strict()
            .expect("isize min should be read")
    );
    assert_eq!(
        "hello 世界",
        reader
            .read_utf8_string_strict(32)
            .expect("strict string should be read")
    );
    assert_eq!(
        "plain",
        reader
            .read_utf8_string(16)
            .expect("non-strict string should be read")
    );
    let position = reader.get_ref().position() as usize;
    assert_eq!(0x7f, reader.get_mut().get_ref()[position]);

    let cursor = reader.into_inner();
    assert_eq!(cursor.get_ref().len() as u64 - 1, cursor.position());
}

#[test]
fn test_leb128_reader_writer_delegate_raw_io_and_accessors() {
    let mut writer = Leb128Writer::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    let mut reader = Leb128Reader::new(Cursor::new(writer.into_inner()));
    let mut bytes = [0; 2];
    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}

#[test]
fn test_leb128_reader_rejects_non_canonical_values_and_strings() {
    let mut reader = Leb128Reader::new(Cursor::new([0x80, 0x00]));
    let error = reader
        .read_uleb_u8_strict()
        .expect_err("non-canonical u8 should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());

    let mut reader = Leb128Reader::new(Cursor::new([0x81, 0x00, b'a']));
    let error = reader
        .read_utf8_string_strict(16)
        .expect_err("non-canonical string length should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}
