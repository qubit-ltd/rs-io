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
    BinaryReadExt,
    BinaryWriteExt,
};

#[test]
fn test_binary_read_write_ext_round_trips_big_endian_scalars() {
    let mut output = Vec::new();

    output.write_u8(0x12).expect("u8 should be written");
    output.write_i8(-2).expect("i8 should be written");
    output.write_u16_be(0x1234).expect("u16 should be written");
    output.write_i16_be(-0x1234).expect("i16 should be written");
    output
        .write_u32_be(0x1234_5678)
        .expect("u32 should be written");
    output
        .write_i32_be(-0x0123_4567)
        .expect("i32 should be written");
    output
        .write_u64_be(0x0123_4567_89ab_cdef)
        .expect("u64 should be written");
    output
        .write_i64_be(-0x0123_4567_89ab_cdef)
        .expect("i64 should be written");
    output.write_f32_be(12.5).expect("f32 should be written");
    output.write_f64_be(-25.25).expect("f64 should be written");

    let mut input = Cursor::new(output);
    assert_eq!(0x12, input.read_u8().expect("u8 should be read"));
    assert_eq!(-2, input.read_i8().expect("i8 should be read"));
    assert_eq!(0x1234, input.read_u16_be().expect("u16 should be read"));
    assert_eq!(-0x1234, input.read_i16_be().expect("i16 should be read"));
    assert_eq!(
        0x1234_5678,
        input.read_u32_be().expect("u32 should be read")
    );
    assert_eq!(
        -0x0123_4567,
        input.read_i32_be().expect("i32 should be read")
    );
    assert_eq!(
        0x0123_4567_89ab_cdef,
        input.read_u64_be().expect("u64 should be read")
    );
    assert_eq!(
        -0x0123_4567_89ab_cdef,
        input.read_i64_be().expect("i64 should be read")
    );
    assert_eq!(12.5, input.read_f32_be().expect("f32 should be read"));
    assert_eq!(-25.25, input.read_f64_be().expect("f64 should be read"));
}

#[test]
fn test_binary_read_ext_reports_unexpected_eof() {
    let mut input = Cursor::new([0x12, 0x34, 0x56]);

    let error = input
        .read_u32_be()
        .expect_err("truncated u32 should report EOF");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_binary_read_ext_reports_unexpected_eof_for_all_scalar_methods() {
    let mut input = Cursor::new([]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input.read_u8().expect_err("u8 should report EOF").kind()
    );

    let mut input = Cursor::new([]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input.read_i8().expect_err("i8 should report EOF").kind()
    );

    let mut input = Cursor::new([0x12]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u16_be()
            .expect_err("u16 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u16_le()
            .expect_err("u16 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i16_be()
            .expect_err("i16 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i16_le()
            .expect_err("i16 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u32_le()
            .expect_err("u32 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i32_be()
            .expect_err("i32 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i32_le()
            .expect_err("i32 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u64_be()
            .expect_err("u64 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u64_le()
            .expect_err("u64 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i64_be()
            .expect_err("i64 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_i64_le()
            .expect_err("i64 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_f32_be()
            .expect_err("f32 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_f32_le()
            .expect_err("f32 little endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_f64_be()
            .expect_err("f64 big endian should report EOF")
            .kind()
    );

    let mut input = Cursor::new([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_f64_le()
            .expect_err("f64 little endian should report EOF")
            .kind()
    );
}

#[test]
fn test_binary_read_write_ext_round_trips_little_endian_scalars() {
    let mut output = Vec::new();
    let i16_value = -0x1234_i16;
    let i32_value = -0x0123_4567_i32;
    let i64_value = -0x0123_4567_89ab_cdef_i64;
    let f32_value = 12.5_f32;
    let f64_value = -25.25_f64;

    output
        .write_u16_le(0x1234)
        .expect("u16 little endian should be written");
    output
        .write_i16_le(i16_value)
        .expect("i16 little endian should be written");
    output
        .write_u32_le(0x1234_5678)
        .expect("u32 little endian should be written");
    output
        .write_i32_le(i32_value)
        .expect("i32 little endian should be written");
    output
        .write_u64_le(0x0123_4567_89ab_cdef)
        .expect("u64 little endian should be written");
    output
        .write_i64_le(i64_value)
        .expect("i64 little endian should be written");
    output
        .write_f32_le(f32_value)
        .expect("f32 little endian should be written");
    output
        .write_f64_le(f64_value)
        .expect("f64 little endian should be written");

    let mut expected = Vec::new();
    expected.extend_from_slice(&0x1234_u16.to_le_bytes());
    expected.extend_from_slice(&i16_value.to_le_bytes());
    expected.extend_from_slice(&0x1234_5678_u32.to_le_bytes());
    expected.extend_from_slice(&i32_value.to_le_bytes());
    expected.extend_from_slice(&0x0123_4567_89ab_cdef_u64.to_le_bytes());
    expected.extend_from_slice(&i64_value.to_le_bytes());
    expected.extend_from_slice(&f32_value.to_bits().to_le_bytes());
    expected.extend_from_slice(&f64_value.to_bits().to_le_bytes());
    assert_eq!(expected, output);

    let mut input = Cursor::new(output);
    assert_eq!(
        0x1234,
        input
            .read_u16_le()
            .expect("u16 little endian should be read")
    );
    assert_eq!(
        i16_value,
        input
            .read_i16_le()
            .expect("i16 little endian should be read")
    );
    assert_eq!(
        0x1234_5678,
        input
            .read_u32_le()
            .expect("u32 little endian should be read")
    );
    assert_eq!(
        i32_value,
        input
            .read_i32_le()
            .expect("i32 little endian should be read")
    );
    assert_eq!(
        0x0123_4567_89ab_cdef,
        input
            .read_u64_le()
            .expect("u64 little endian should be read")
    );
    assert_eq!(
        i64_value,
        input
            .read_i64_le()
            .expect("i64 little endian should be read")
    );
    assert_eq!(
        f32_value,
        input
            .read_f32_le()
            .expect("f32 little endian should be read")
    );
    assert_eq!(
        f64_value,
        input
            .read_f64_le()
            .expect("f64 little endian should be read")
    );
}
