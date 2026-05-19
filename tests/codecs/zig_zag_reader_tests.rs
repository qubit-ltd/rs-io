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
    ZigZagReader,
    ZigZagWriteExt,
};

#[test]
fn test_zig_zag_reader_reads_signed_values() {
    let mut bytes = Vec::new();
    bytes.write_zigzag_i8(-1).expect("i8 should be written");
    bytes
        .write_zigzag_i16(i16::MIN)
        .expect("i16 should be written");
    bytes
        .write_zigzag_i32(-123_456)
        .expect("i32 should be written");
    bytes
        .write_zigzag_i64(i64::MAX)
        .expect("i64 should be written");
    bytes
        .write_zigzag_i128(i128::MIN)
        .expect("i128 should be written");
    bytes
        .write_zigzag_isize(isize::MIN)
        .expect("isize should be written");
    bytes.push(0);

    let mut reader = ZigZagReader::with_strict(Cursor::new(bytes), true);
    assert!(reader.is_strict());

    assert_eq!(-1, reader.read_i8().expect("i8 should be read"));
    assert_eq!(i16::MIN, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-123_456, reader.read_i32().expect("i32 should be read"));
    assert_eq!(i64::MAX, reader.read_i64().expect("i64 should be read"));
    assert_eq!(i128::MIN, reader.read_i128().expect("i128 should be read"));
    assert_eq!(
        isize::MIN,
        reader.read_isize().expect("isize should be read")
    );
    let position = reader.get_ref().position() as usize;
    assert_eq!(0, reader.get_mut().get_ref()[position]);

    let cursor = reader.into_inner();
    assert_eq!(cursor.get_ref().len() as u64 - 1, cursor.position());
}

#[test]
fn test_zig_zag_reader_delegates_raw_read() {
    let mut reader = ZigZagReader::new(Cursor::new(vec![0x01, 0x02]));
    let mut bytes = [0; 2];

    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}

#[test]
fn test_zig_zag_reader_default_non_strict_accepts_non_canonical_underlying_uleb() {
    let mut reader = ZigZagReader::new(Cursor::new([0x80, 0x00]));
    assert!(!reader.is_strict());

    assert_eq!(
        0,
        reader
            .read_i8()
            .expect("non-strict ZigZag value should be read")
    );
}

#[test]
fn test_zig_zag_reader_rejects_non_canonical_underlying_uleb() {
    let mut reader = ZigZagReader::new(Cursor::new([0x80, 0x00]));
    assert!(!reader.is_strict());
    reader.set_strict(true);
    assert!(reader.is_strict());

    let error = reader
        .read_i8()
        .expect_err("non-canonical underlying ULEB should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
