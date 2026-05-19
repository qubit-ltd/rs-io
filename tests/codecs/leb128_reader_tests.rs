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
};

use qubit_io::{
    Leb128Reader,
    Leb128WriteExt,
    StringWriteExt,
};

#[test]
fn test_leb128_reader_reads_boundary_values_and_strings() {
    let mut bytes = Vec::new();
    bytes.write_uleb_u8(0).expect("u8 zero should be written");
    bytes
        .write_uleb_u8(u8::MAX)
        .expect("u8 max should be written");
    bytes
        .write_uleb_u16(u16::MAX)
        .expect("u16 max should be written");
    bytes
        .write_uleb_u32(u32::MAX)
        .expect("u32 max should be written");
    bytes
        .write_uleb_u64(u64::MAX)
        .expect("u64 max should be written");
    bytes
        .write_uleb_u128(u128::MAX)
        .expect("u128 max should be written");
    bytes
        .write_uleb_usize(usize::MAX)
        .expect("usize max should be written");
    bytes.write_sleb_i8(0).expect("i8 zero should be written");
    bytes
        .write_sleb_i8(i8::MIN)
        .expect("i8 min should be written");
    bytes
        .write_sleb_i16(i16::MAX)
        .expect("i16 max should be written");
    bytes
        .write_sleb_i32(i32::MIN)
        .expect("i32 min should be written");
    bytes
        .write_sleb_i64(i64::MAX)
        .expect("i64 max should be written");
    bytes
        .write_sleb_i128(i128::MIN)
        .expect("i128 min should be written");
    bytes
        .write_sleb_isize(isize::MIN)
        .expect("isize min should be written");
    bytes
        .write_utf8_string_uleb("hello 世界")
        .expect("string should be written");
    bytes
        .write_utf8_string_uleb("plain")
        .expect("second string should be written");
    bytes.push(0x7f);

    let mut reader = Leb128Reader::with_strict(Cursor::new(bytes), true);
    assert!(reader.is_strict());

    assert_eq!(0, reader.read_u8().expect("u8 zero should be read"));
    assert_eq!(u8::MAX, reader.read_u8().expect("u8 max should be read"));
    assert_eq!(u16::MAX, reader.read_u16().expect("u16 max should be read"));
    assert_eq!(u32::MAX, reader.read_u32().expect("u32 max should be read"));
    assert_eq!(u64::MAX, reader.read_u64().expect("u64 max should be read"));
    assert_eq!(
        u128::MAX,
        reader.read_u128().expect("u128 max should be read")
    );
    assert_eq!(
        usize::MAX,
        reader.read_usize().expect("usize max should be read")
    );
    assert_eq!(0, reader.read_i8().expect("i8 zero should be read"));
    assert_eq!(i8::MIN, reader.read_i8().expect("i8 min should be read"));
    assert_eq!(i16::MAX, reader.read_i16().expect("i16 max should be read"));
    assert_eq!(i32::MIN, reader.read_i32().expect("i32 min should be read"));
    assert_eq!(i64::MAX, reader.read_i64().expect("i64 max should be read"));
    assert_eq!(
        i128::MIN,
        reader.read_i128().expect("i128 min should be read")
    );
    assert_eq!(
        isize::MIN,
        reader.read_isize().expect("isize min should be read")
    );
    assert_eq!(
        "hello 世界",
        reader.read_utf8_string(32).expect("string should be read")
    );
    assert_eq!(
        "plain",
        reader
            .read_utf8_string(16)
            .expect("second string should be read")
    );
    let position = reader.get_ref().position() as usize;
    assert_eq!(0x7f, reader.get_mut().get_ref()[position]);

    let cursor = reader.into_inner();
    assert_eq!(cursor.get_ref().len() as u64 - 1, cursor.position());
}

#[test]
fn test_leb128_reader_delegates_raw_read() {
    let mut reader = Leb128Reader::new(Cursor::new(vec![0x01, 0x02]));
    let mut bytes = [0; 2];

    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}

#[test]
fn test_leb128_reader_default_non_strict_accepts_non_canonical_values() {
    let mut reader = Leb128Reader::new(Cursor::new(vec![0x80, 0x00, 0x81, 0x00, b'a']));
    assert!(!reader.is_strict());

    assert_eq!(0, reader.read_u8().expect("non-strict u8 should be read"));
    assert_eq!(
        "a",
        reader
            .read_utf8_string(4)
            .expect("non-strict string should be read")
    );
}

#[test]
fn test_leb128_reader_rejects_non_canonical_values_and_strings() {
    let mut reader = Leb128Reader::new(Cursor::new(vec![0x80, 0x00]));
    assert!(!reader.is_strict());
    reader.set_strict(true);
    assert!(reader.is_strict());

    let error = reader.read_u8().expect_err("non-canonical u8 should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());

    let mut reader = Leb128Reader::with_strict(Cursor::new(vec![0x81, 0x00, b'a']), true);
    let error = reader
        .read_utf8_string(16)
        .expect_err("non-canonical string length should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}
