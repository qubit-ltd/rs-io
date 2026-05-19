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
    Write,
};

use qubit_io::{
    ZigZagReadExt,
    ZigZagWriter,
};

#[test]
fn test_zig_zag_writer_writes_signed_values() {
    let mut writer = ZigZagWriter::new(Vec::new());

    writer.write_i8(-1).expect("i8 should be written");
    writer.write_i16(i16::MIN).expect("i16 should be written");
    writer.write_i32(-123_456).expect("i32 should be written");
    writer.write_i64(i64::MAX).expect("i64 should be written");
    writer
        .write_i128(i128::MIN)
        .expect("i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize should be written");
    writer.get_mut().push(0);

    assert_eq!(Some(&0), writer.get_ref().last());

    let mut reader = Cursor::new(writer.into_inner());
    assert_eq!(-1, reader.read_zigzag_i8().expect("i8 should be read"));
    assert_eq!(
        i16::MIN,
        reader.read_zigzag_i16().expect("i16 should be read")
    );
    assert_eq!(
        -123_456,
        reader.read_zigzag_i32().expect("i32 should be read")
    );
    assert_eq!(
        i64::MAX,
        reader.read_zigzag_i64().expect("i64 should be read")
    );
    assert_eq!(
        i128::MIN,
        reader.read_zigzag_i128().expect("i128 should be read")
    );
    assert_eq!(
        isize::MIN,
        reader.read_zigzag_isize().expect("isize should be read")
    );
    assert_eq!(0, reader.get_ref()[reader.position() as usize]);
}

#[test]
fn test_zig_zag_writer_delegates_raw_write_and_flush() {
    let mut writer = ZigZagWriter::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    assert_eq!(vec![0x01, 0x02], writer.into_inner());
}
