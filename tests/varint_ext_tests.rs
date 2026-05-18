/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0
 *
 ******************************************************************************/
use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    Leb128IntReadExt,
    Leb128IntWriteExt,
    ZigZagIntReadExt,
    ZigZagIntWriteExt,
};

#[test]
fn test_leb128_int_ext_round_trips_unsigned_values() {
    let mut output = Vec::new();

    output
        .write_uleb_u32(0)
        .expect("zero u32 LEB128 value should be written");
    output
        .write_uleb_u32(127)
        .expect("single-byte u32 LEB128 value should be written");
    output
        .write_uleb_u32(128)
        .expect("two-byte u32 LEB128 value should be written");
    output
        .write_uleb_u32(u32::MAX)
        .expect("max u32 LEB128 value should be written");
    output
        .write_uleb_u64(u64::MAX)
        .expect("max u64 LEB128 value should be written");

    assert_eq!(
        vec![
            0x00, 0x7f, 0x80, 0x01, 0xff, 0xff, 0xff, 0xff, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0x01,
        ],
        output
    );

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_uleb_u32().expect("zero should be read"));
    assert_eq!(127, input.read_uleb_u32().expect("127 should be read"));
    assert_eq!(128, input.read_uleb_u32().expect("128 should be read"));
    assert_eq!(
        u32::MAX,
        input.read_uleb_u32().expect("max u32 should be read")
    );
    assert_eq!(
        u64::MAX,
        input.read_uleb_u64().expect("max u64 should be read")
    );
}

#[test]
fn test_leb128_int_read_ext_rejects_u32_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0x10]);

    let error = input
        .read_uleb_u32()
        .expect_err("overflowing u32 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_read_ext_rejects_unterminated_u32() {
    let mut input = Cursor::new([0x80, 0x80, 0x80, 0x80, 0x80]);

    let error = input
        .read_uleb_u32()
        .expect_err("unterminated u32 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_read_ext_rejects_u64_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]);

    let error = input
        .read_uleb_u64()
        .expect_err("overflowing u64 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_read_ext_rejects_unterminated_u64() {
    let mut input = Cursor::new([0x80; 10]);

    let error = input
        .read_uleb_u64()
        .expect_err("unterminated u64 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_read_ext_rejects_usize_overflow() {
    let bytes = if usize::BITS == 64 {
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]
    } else {
        vec![0xff, 0xff, 0xff, 0xff, 0x10]
    };
    let mut input = Cursor::new(bytes);

    let error = input
        .read_uleb_usize()
        .expect_err("overflowing usize LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_read_ext_reports_unexpected_eof() {
    let mut input = Cursor::new([0x80]);

    let error = input
        .read_uleb_u64()
        .expect_err("truncated LEB128 value should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_leb128_int_ext_round_trips_usize_values() {
    let mut output = Vec::new();

    output
        .write_uleb_usize(usize::MAX)
        .expect("max usize LEB128 value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        usize::MAX,
        input
            .read_uleb_usize()
            .expect("max usize LEB128 value should be read")
    );
}

#[test]
fn test_leb128_int_ext_round_trips_signed_values() {
    let mut output = Vec::new();

    output
        .write_sleb_i32(0)
        .expect("zero i32 SLEB128 value should be written");
    output
        .write_sleb_i32(-1)
        .expect("negative one i32 SLEB128 value should be written");
    output
        .write_sleb_i32(624_485)
        .expect("positive i32 SLEB128 value should be written");
    output
        .write_sleb_i32(-624_485)
        .expect("negative i32 SLEB128 value should be written");
    output
        .write_sleb_i64(i64::MIN)
        .expect("minimum i64 SLEB128 value should be written");

    assert_eq!(
        vec![
            0x00, 0x7f, 0xe5, 0x8e, 0x26, 0x9b, 0xf1, 0x59, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            0x80, 0x80, 0x80, 0x7f,
        ],
        output
    );

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_sleb_i32().expect("zero should be read"));
    assert_eq!(-1, input.read_sleb_i32().expect("-1 should be read"));
    assert_eq!(
        624_485,
        input.read_sleb_i32().expect("positive i32 should be read")
    );
    assert_eq!(
        -624_485,
        input.read_sleb_i32().expect("negative i32 should be read")
    );
    assert_eq!(
        i64::MIN,
        input.read_sleb_i64().expect("minimum i64 should be read")
    );
}

#[test]
fn test_leb128_int_read_ext_rejects_i32_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0x0f]);

    let error = input
        .read_sleb_i32()
        .expect_err("overflowing i32 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_int_ext_round_trips_isize_values() {
    let mut output = Vec::new();

    output
        .write_sleb_isize(isize::MIN)
        .expect("minimum isize SLEB128 value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        isize::MIN,
        input
            .read_sleb_isize()
            .expect("minimum isize should be read")
    );
}

#[test]
fn test_zigzag_int_ext_round_trips_signed_values() {
    let mut output = Vec::new();

    output
        .write_zigzag_i32(0)
        .expect("zero i32 ZigZag value should be written");
    output
        .write_zigzag_i32(-1)
        .expect("negative one i32 ZigZag value should be written");
    output
        .write_zigzag_i32(1)
        .expect("positive one i32 ZigZag value should be written");
    output
        .write_zigzag_i32(i32::MIN)
        .expect("minimum i32 ZigZag value should be written");
    output
        .write_zigzag_i64(i64::MIN)
        .expect("minimum i64 ZigZag value should be written");

    assert_eq!(
        vec![
            0x00, 0x01, 0x02, 0xff, 0xff, 0xff, 0xff, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x01,
        ],
        output
    );

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_zigzag_i32().expect("zero should be read"));
    assert_eq!(-1, input.read_zigzag_i32().expect("-1 should be read"));
    assert_eq!(1, input.read_zigzag_i32().expect("1 should be read"));
    assert_eq!(
        i32::MIN,
        input.read_zigzag_i32().expect("minimum i32 should be read")
    );
    assert_eq!(
        i64::MIN,
        input.read_zigzag_i64().expect("minimum i64 should be read")
    );
}

#[test]
fn test_zigzag_int_ext_round_trips_isize_values() {
    let mut output = Vec::new();

    output
        .write_zigzag_isize(isize::MIN)
        .expect("minimum isize ZigZag value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        isize::MIN,
        input
            .read_zigzag_isize()
            .expect("minimum isize should be read")
    );
}
