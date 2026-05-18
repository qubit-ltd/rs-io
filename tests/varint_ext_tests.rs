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
    VarIntReadExt,
    VarIntWriteExt,
};

#[test]
fn test_varint_ext_round_trips_unsigned_values() {
    let mut output = Vec::new();

    output
        .write_uvar_u32(0)
        .expect("zero u32 varint should be written");
    output
        .write_uvar_u32(127)
        .expect("single-byte u32 varint should be written");
    output
        .write_uvar_u32(128)
        .expect("two-byte u32 varint should be written");
    output
        .write_uvar_u32(u32::MAX)
        .expect("max u32 varint should be written");
    output
        .write_uvar_u64(u64::MAX)
        .expect("max u64 varint should be written");

    assert_eq!(
        vec![
            0x00, 0x7f, 0x80, 0x01, 0xff, 0xff, 0xff, 0xff, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0x01,
        ],
        output
    );

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_uvar_u32().expect("zero should be read"));
    assert_eq!(127, input.read_uvar_u32().expect("127 should be read"));
    assert_eq!(128, input.read_uvar_u32().expect("128 should be read"));
    assert_eq!(
        u32::MAX,
        input.read_uvar_u32().expect("max u32 should be read")
    );
    assert_eq!(
        u64::MAX,
        input.read_uvar_u64().expect("max u64 should be read")
    );
}

#[test]
fn test_varint_read_ext_rejects_u32_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0x10]);

    let error = input
        .read_uvar_u32()
        .expect_err("overflowing u32 varint should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_varint_read_ext_rejects_unterminated_u32() {
    let mut input = Cursor::new([0x80, 0x80, 0x80, 0x80, 0x80]);

    let error = input
        .read_uvar_u32()
        .expect_err("unterminated u32 varint should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_varint_read_ext_rejects_u64_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]);

    let error = input
        .read_uvar_u64()
        .expect_err("overflowing u64 varint should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_varint_read_ext_rejects_unterminated_u64() {
    let mut input = Cursor::new([0x80; 10]);

    let error = input
        .read_uvar_u64()
        .expect_err("unterminated u64 varint should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_varint_read_ext_rejects_usize_overflow() {
    let bytes = if usize::BITS == 64 {
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]
    } else {
        vec![0xff, 0xff, 0xff, 0xff, 0x10]
    };
    let mut input = Cursor::new(bytes);

    let error = input
        .read_uvar_usize()
        .expect_err("overflowing usize varint should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_varint_read_ext_reports_unexpected_eof() {
    let mut input = Cursor::new([0x80]);

    let error = input
        .read_uvar_u64()
        .expect_err("truncated varint should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_varint_ext_round_trips_usize_values() {
    let mut output = Vec::new();

    output
        .write_uvar_usize(usize::MAX)
        .expect("max usize varint should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        usize::MAX,
        input
            .read_uvar_usize()
            .expect("max usize varint should be read")
    );
}
