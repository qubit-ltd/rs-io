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
    ZigZagReader,
    ZigZagWriter,
};

#[test]
fn test_zig_zag_reader_writer_round_trips_signed_values() {
    let mut writer = ZigZagWriter::new(Vec::new());

    writer.write_zigzag_i8(-1).expect("i8 should be written");
    writer
        .write_zigzag_i16(i16::MIN)
        .expect("i16 should be written");
    writer
        .write_zigzag_i32(-123_456)
        .expect("i32 should be written");
    writer
        .write_zigzag_i64(i64::MAX)
        .expect("i64 should be written");
    writer
        .write_zigzag_i128(i128::MIN)
        .expect("i128 should be written");
    writer
        .write_zigzag_isize(isize::MIN)
        .expect("isize should be written");
    writer.get_mut().push(0);

    assert_eq!(Some(&0), writer.get_ref().last());

    let bytes = writer.into_inner();
    let mut reader = ZigZagReader::new(Cursor::new(bytes));

    assert_eq!(-1, reader.read_zigzag_i8().expect("i8 should be read"));
    assert_eq!(
        i16::MIN,
        reader.read_zigzag_i16_strict().expect("i16 should be read")
    );
    assert_eq!(
        -123_456,
        reader.read_zigzag_i32_strict().expect("i32 should be read")
    );
    assert_eq!(
        i64::MAX,
        reader.read_zigzag_i64_strict().expect("i64 should be read")
    );
    assert_eq!(
        i128::MIN,
        reader
            .read_zigzag_i128_strict()
            .expect("i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        reader
            .read_zigzag_isize_strict()
            .expect("isize should be read")
    );
    let position = reader.get_ref().position() as usize;
    assert_eq!(0, reader.get_mut().get_ref()[position]);

    let cursor = reader.into_inner();
    assert_eq!(cursor.get_ref().len() as u64 - 1, cursor.position());
}

#[test]
fn test_zig_zag_reader_writer_delegate_raw_io_and_accessors() {
    let mut writer = ZigZagWriter::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    let mut reader = ZigZagReader::new(Cursor::new(writer.into_inner()));
    let mut bytes = [0; 2];
    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([0x01, 0x02], bytes);
}

#[test]
fn test_zig_zag_reader_rejects_non_canonical_underlying_uleb() {
    let mut reader = ZigZagReader::new(Cursor::new([0x80, 0x00]));

    let error = reader
        .read_zigzag_i8_strict()
        .expect_err("non-canonical underlying ULEB should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
