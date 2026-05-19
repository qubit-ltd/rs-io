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
    Leb128ReadExt,
    Leb128WriteExt,
    ZigZagReadExt,
    ZigZagWriteExt,
};

#[test]
fn test_leb128_ext_round_trips_u8_values() {
    let mut output = Vec::new();

    output
        .write_uleb_u8(0)
        .expect("zero u8 LEB128 value should be written");
    output
        .write_uleb_u8(127)
        .expect("single-byte u8 LEB128 value should be written");
    output
        .write_uleb_u8(128)
        .expect("two-byte u8 LEB128 value should be written");
    output
        .write_uleb_u8(u8::MAX)
        .expect("max u8 LEB128 value should be written");

    assert_eq!(vec![0x00, 0x7f, 0x80, 0x01, 0xff, 0x01], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_uleb_u8().expect("zero should be read"));
    assert_eq!(127, input.read_uleb_u8().expect("127 should be read"));
    assert_eq!(128, input.read_uleb_u8().expect("128 should be read"));
    assert_eq!(
        u8::MAX,
        input.read_uleb_u8().expect("max u8 should be read")
    );
}

#[test]
fn test_leb128_ext_round_trips_u16_values() {
    let mut output = Vec::new();

    output
        .write_uleb_u16(0)
        .expect("zero u16 LEB128 value should be written");
    output
        .write_uleb_u16(127)
        .expect("single-byte u16 LEB128 value should be written");
    output
        .write_uleb_u16(128)
        .expect("two-byte u16 LEB128 value should be written");
    output
        .write_uleb_u16(u16::MAX)
        .expect("max u16 LEB128 value should be written");

    assert_eq!(vec![0x00, 0x7f, 0x80, 0x01, 0xff, 0xff, 0x03], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_uleb_u16().expect("zero should be read"));
    assert_eq!(127, input.read_uleb_u16().expect("127 should be read"));
    assert_eq!(128, input.read_uleb_u16().expect("128 should be read"));
    assert_eq!(
        u16::MAX,
        input.read_uleb_u16().expect("max u16 should be read")
    );
}

#[test]
fn test_leb128_ext_round_trips_unsigned_values() {
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
fn test_leb128_ext_reads_single_byte_u64_from_array_cursor() {
    let mut input = Cursor::new([0x7f]);

    assert_eq!(
        127,
        input
            .read_uleb_u64()
            .expect("single-byte u64 LEB128 value should be read")
    );
}

#[test]
fn test_leb128_ext_round_trips_u128_values() {
    let mut output = Vec::new();

    output
        .write_uleb_u128(u128::MAX)
        .expect("max u128 LEB128 value should be written");

    let mut expected = vec![0xff; 18];
    expected.push(0x03);
    assert_eq!(expected, output);

    let mut input = Cursor::new(output);
    assert_eq!(
        u128::MAX,
        input.read_uleb_u128().expect("max u128 should be read")
    );
}

#[test]
fn test_leb128_read_ext_rejects_u8_overflow() {
    let mut input = Cursor::new([0x80, 0x02]);

    let error = input
        .read_uleb_u8()
        .expect_err("overflowing u8 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_u16_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0x04]);

    let error = input
        .read_uleb_u16()
        .expect_err("overflowing u16 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_u128_overflow() {
    let mut bytes = vec![0xff; 18];
    bytes.push(0x04);
    let mut input = Cursor::new(bytes);

    let error = input
        .read_uleb_u128()
        .expect_err("overflowing u128 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_unterminated_u128() {
    let mut input = Cursor::new([0x80; 19]);

    let error = input
        .read_uleb_u128()
        .expect_err("unterminated u128 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_unterminated_u16() {
    let mut input = Cursor::new([0x80, 0x80, 0x80]);

    let error = input
        .read_uleb_u16()
        .expect_err("unterminated u16 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_u32_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0x10]);

    let error = input
        .read_uleb_u32()
        .expect_err("overflowing u32 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_unterminated_u32() {
    let mut input = Cursor::new([0x80, 0x80, 0x80, 0x80, 0x80]);

    let error = input
        .read_uleb_u32()
        .expect_err("unterminated u32 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_u64_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]);

    let error = input
        .read_uleb_u64()
        .expect_err("overflowing u64 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_unterminated_u64() {
    let mut input = Cursor::new([0x80; 10]);

    let error = input
        .read_uleb_u64()
        .expect_err("unterminated u64 LEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_usize_overflow() {
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
fn test_leb128_read_ext_reports_unexpected_eof() {
    let mut input = Cursor::new([0x80]);

    let error = input
        .read_uleb_u64()
        .expect_err("truncated LEB128 value should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_leb128_ext_round_trips_usize_values() {
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
fn test_leb128_ext_round_trips_i8_values() {
    let mut output = Vec::new();

    output
        .write_sleb_i8(0)
        .expect("zero i8 SLEB128 value should be written");
    output
        .write_sleb_i8(-1)
        .expect("negative one i8 SLEB128 value should be written");
    output
        .write_sleb_i8(i8::MAX)
        .expect("maximum i8 SLEB128 value should be written");
    output
        .write_sleb_i8(i8::MIN)
        .expect("minimum i8 SLEB128 value should be written");

    assert_eq!(vec![0x00, 0x7f, 0xff, 0x00, 0x80, 0x7f], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_sleb_i8().expect("zero should be read"));
    assert_eq!(-1, input.read_sleb_i8().expect("-1 should be read"));
    assert_eq!(
        i8::MAX,
        input.read_sleb_i8().expect("maximum i8 should be read")
    );
    assert_eq!(
        i8::MIN,
        input.read_sleb_i8().expect("minimum i8 should be read")
    );
}

#[test]
fn test_leb128_ext_round_trips_i16_values() {
    let mut output = Vec::new();

    output
        .write_sleb_i16(0)
        .expect("zero i16 SLEB128 value should be written");
    output
        .write_sleb_i16(-1)
        .expect("negative one i16 SLEB128 value should be written");
    output
        .write_sleb_i16(i16::MAX)
        .expect("maximum i16 SLEB128 value should be written");
    output
        .write_sleb_i16(i16::MIN)
        .expect("minimum i16 SLEB128 value should be written");

    assert_eq!(vec![0x00, 0x7f, 0xff, 0xff, 0x01, 0x80, 0x80, 0x7e], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_sleb_i16().expect("zero should be read"));
    assert_eq!(-1, input.read_sleb_i16().expect("-1 should be read"));
    assert_eq!(
        i16::MAX,
        input.read_sleb_i16().expect("maximum i16 should be read")
    );
    assert_eq!(
        i16::MIN,
        input.read_sleb_i16().expect("minimum i16 should be read")
    );
}

#[test]
fn test_leb128_ext_round_trips_signed_values() {
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
fn test_leb128_ext_round_trips_i128_values() {
    let mut output = Vec::new();

    output
        .write_sleb_i128(i128::MAX)
        .expect("maximum i128 SLEB128 value should be written");
    output
        .write_sleb_i128(i128::MIN)
        .expect("minimum i128 SLEB128 value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        i128::MAX,
        input.read_sleb_i128().expect("maximum i128 should be read")
    );
    assert_eq!(
        i128::MIN,
        input.read_sleb_i128().expect("minimum i128 should be read")
    );
}

#[test]
fn test_leb128_read_ext_rejects_i8_overflow() {
    let mut input = Cursor::new([0x80, 0x02]);

    let error = input
        .read_sleb_i8()
        .expect_err("overflowing i8 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_i16_overflow() {
    let mut input = Cursor::new([0x80, 0x80, 0x02]);

    let error = input
        .read_sleb_i16()
        .expect_err("overflowing i16 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_unterminated_i8() {
    let mut input = Cursor::new([0x80, 0x80]);

    let error = input
        .read_sleb_i8()
        .expect_err("unterminated i8 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_i32_overflow() {
    let mut input = Cursor::new([0xff, 0xff, 0xff, 0xff, 0x0f]);

    let error = input
        .read_sleb_i32()
        .expect_err("overflowing i32 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_rejects_i128_overflow() {
    let mut bytes = vec![0xff; 18];
    bytes.push(0x03);
    let mut input = Cursor::new(bytes);

    let error = input
        .read_sleb_i128()
        .expect_err("overflowing i128 SLEB128 value should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_ext_round_trips_isize_values() {
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
fn test_leb128_read_ext_strict_accepts_canonical_unsigned_values() {
    let mut input = Cursor::new([0x00, 0x80, 0x01]);

    assert_eq!(
        0,
        input
            .read_uleb_u8_strict()
            .expect("canonical u8 zero should be read")
    );
    assert_eq!(
        128,
        input
            .read_uleb_u16_strict()
            .expect("canonical u16 value should be read")
    );
}

#[test]
fn test_leb128_read_ext_strict_accepts_canonical_wide_unsigned_values() {
    let mut output = Vec::new();
    output
        .write_uleb_u64(u64::MAX)
        .expect("max u64 should be written");
    output
        .write_uleb_usize(usize::MAX)
        .expect("max usize should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        u64::MAX,
        input
            .read_uleb_u64_strict()
            .expect("canonical u64 should be read")
    );
    assert_eq!(
        usize::MAX,
        input
            .read_uleb_usize_strict()
            .expect("canonical usize should be read")
    );
}

#[test]
fn test_leb128_read_ext_strict_rejects_noncanonical_unsigned_values() {
    let mut input = Cursor::new([0x80, 0x00]);

    let error = input
        .read_uleb_u32_strict()
        .expect_err("non-canonical unsigned LEB128 should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_strict_accepts_canonical_signed_values() {
    let mut input = Cursor::new([0x7f, 0xff, 0x00]);

    assert_eq!(
        -1,
        input
            .read_sleb_i8_strict()
            .expect("canonical i8 negative one should be read")
    );
    assert_eq!(
        127,
        input
            .read_sleb_i16_strict()
            .expect("canonical i16 value should be read")
    );
}

#[test]
fn test_leb128_read_ext_strict_accepts_canonical_wide_signed_values() {
    let mut output = Vec::new();
    output
        .write_sleb_i64(i64::MIN)
        .expect("min i64 should be written");
    output
        .write_sleb_isize(isize::MIN)
        .expect("min isize should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        i64::MIN,
        input
            .read_sleb_i64_strict()
            .expect("canonical i64 should be read")
    );
    assert_eq!(
        isize::MIN,
        input
            .read_sleb_isize_strict()
            .expect("canonical isize should be read")
    );
}

#[test]
fn test_leb128_read_ext_strict_rejects_noncanonical_signed_values() {
    let mut input = Cursor::new([0xff, 0x7f]);

    let error = input
        .read_sleb_i32_strict()
        .expect_err("non-canonical signed LEB128 should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_leb128_read_ext_strict_round_trips_128_bit_values() {
    let mut output = Vec::new();
    output
        .write_uleb_u128(u128::MAX)
        .expect("max u128 should be written");
    output
        .write_sleb_i128(i128::MIN)
        .expect("min i128 should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        u128::MAX,
        input
            .read_uleb_u128_strict()
            .expect("canonical u128 should be read")
    );
    assert_eq!(
        i128::MIN,
        input
            .read_sleb_i128_strict()
            .expect("canonical i128 should be read")
    );
}

#[test]
fn test_zig_zag_ext_round_trips_i8_values() {
    let mut output = Vec::new();

    output
        .write_zigzag_i8(0)
        .expect("zero i8 ZigZag value should be written");
    output
        .write_zigzag_i8(-1)
        .expect("negative one i8 ZigZag value should be written");
    output
        .write_zigzag_i8(1)
        .expect("positive one i8 ZigZag value should be written");
    output
        .write_zigzag_i8(i8::MIN)
        .expect("minimum i8 ZigZag value should be written");

    assert_eq!(vec![0x00, 0x01, 0x02, 0xff, 0x01], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_zigzag_i8().expect("zero should be read"));
    assert_eq!(-1, input.read_zigzag_i8().expect("-1 should be read"));
    assert_eq!(1, input.read_zigzag_i8().expect("1 should be read"));
    assert_eq!(
        i8::MIN,
        input.read_zigzag_i8().expect("minimum i8 should be read")
    );
}

#[test]
fn test_zig_zag_ext_round_trips_i16_values() {
    let mut output = Vec::new();

    output
        .write_zigzag_i16(0)
        .expect("zero i16 ZigZag value should be written");
    output
        .write_zigzag_i16(-1)
        .expect("negative one i16 ZigZag value should be written");
    output
        .write_zigzag_i16(1)
        .expect("positive one i16 ZigZag value should be written");
    output
        .write_zigzag_i16(i16::MIN)
        .expect("minimum i16 ZigZag value should be written");

    assert_eq!(vec![0x00, 0x01, 0x02, 0xff, 0xff, 0x03], output);

    let mut input = Cursor::new(output);
    assert_eq!(0, input.read_zigzag_i16().expect("zero should be read"));
    assert_eq!(-1, input.read_zigzag_i16().expect("-1 should be read"));
    assert_eq!(1, input.read_zigzag_i16().expect("1 should be read"));
    assert_eq!(
        i16::MIN,
        input.read_zigzag_i16().expect("minimum i16 should be read")
    );
}

#[test]
fn test_zig_zag_ext_round_trips_i128_values() {
    let mut output = Vec::new();

    output
        .write_zigzag_i128(i128::MIN)
        .expect("minimum i128 ZigZag value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        i128::MIN,
        input
            .read_zigzag_i128()
            .expect("minimum i128 should be read")
    );
}

#[test]
fn test_zig_zag_read_ext_strict_rejects_noncanonical_payload() {
    let mut input = Cursor::new([0x80, 0x00]);

    let error = input
        .read_zigzag_i32_strict()
        .expect_err("non-canonical ZigZag payload should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_zig_zag_read_ext_strict_accepts_canonical_values() {
    let mut output = Vec::new();
    output
        .write_zigzag_i8(i8::MIN)
        .expect("minimum i8 ZigZag value should be written");
    output
        .write_zigzag_i16(i16::MIN)
        .expect("minimum i16 ZigZag value should be written");
    output
        .write_zigzag_i32(i32::MIN)
        .expect("minimum i32 ZigZag value should be written");
    output
        .write_zigzag_i64(i64::MIN)
        .expect("minimum i64 ZigZag value should be written");
    output
        .write_zigzag_i128(i128::MIN)
        .expect("minimum i128 ZigZag value should be written");
    output
        .write_zigzag_isize(isize::MIN)
        .expect("minimum isize ZigZag value should be written");

    let mut input = Cursor::new(output);
    assert_eq!(
        i8::MIN,
        input
            .read_zigzag_i8_strict()
            .expect("canonical i8 ZigZag value should be read")
    );
    assert_eq!(
        i16::MIN,
        input
            .read_zigzag_i16_strict()
            .expect("canonical i16 ZigZag value should be read")
    );
    assert_eq!(
        i32::MIN,
        input
            .read_zigzag_i32_strict()
            .expect("canonical i32 ZigZag value should be read")
    );
    assert_eq!(
        i64::MIN,
        input
            .read_zigzag_i64_strict()
            .expect("canonical i64 ZigZag value should be read")
    );
    assert_eq!(
        i128::MIN,
        input
            .read_zigzag_i128_strict()
            .expect("canonical i128 ZigZag value should be read")
    );
    assert_eq!(
        isize::MIN,
        input
            .read_zigzag_isize_strict()
            .expect("canonical isize ZigZag value should be read")
    );
}

#[test]
fn test_zig_zag_ext_round_trips_signed_values() {
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
fn test_zig_zag_ext_round_trips_isize_values() {
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
