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
    Read,
    Result,
};

use crate::Leb128IntReadExt;

/// Extension methods for reading ZigZag encoded signed integers.
///
/// ZigZag maps signed integers to unsigned integers so small negative values
/// still have short varint encodings. The mapped unsigned value is read with
/// unsigned LEB128. The ZigZag mapping follows the Protocol Buffers encoding
/// guide:
/// <https://protobuf.dev/programming-guides/encoding/#signed-integers>.
pub trait ZigZagIntReadExt: Read {
    /// Reads a ZigZag encoded `i16`.
    ///
    /// # Returns
    /// The decoded signed value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed
    /// unsigned LEB128 input, or another I/O error from the underlying reader.
    fn read_zigzag_i16(&mut self) -> Result<i16>;

    /// Reads a ZigZag encoded `i32`.
    ///
    /// # Returns
    /// The decoded signed value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed
    /// unsigned LEB128 input, or another I/O error from the underlying reader.
    fn read_zigzag_i32(&mut self) -> Result<i32>;

    /// Reads a ZigZag encoded `i64`.
    ///
    /// # Returns
    /// The decoded signed value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed
    /// unsigned LEB128 input, or another I/O error from the underlying reader.
    fn read_zigzag_i64(&mut self) -> Result<i64>;

    /// Reads a ZigZag encoded `isize`.
    ///
    /// # Returns
    /// The decoded signed value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed
    /// unsigned LEB128 input, or another I/O error from the underlying reader.
    fn read_zigzag_isize(&mut self) -> Result<isize>;
}

impl<T> ZigZagIntReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_zigzag_i16(&mut self) -> Result<i16> {
        self.read_uleb_u16().map(decode_zigzag_u16)
    }

    #[inline]
    fn read_zigzag_i32(&mut self) -> Result<i32> {
        self.read_uleb_u32().map(decode_zigzag_u32)
    }

    #[inline]
    fn read_zigzag_i64(&mut self) -> Result<i64> {
        self.read_uleb_u64().map(decode_zigzag_u64)
    }

    #[inline]
    fn read_zigzag_isize(&mut self) -> Result<isize> {
        self.read_uleb_usize().map(decode_zigzag_usize)
    }
}

/// Decodes a ZigZag mapped `u16`.
///
/// # Parameters
/// - `value`: ZigZag mapped unsigned value.
///
/// # Returns
/// Decoded signed value.
fn decode_zigzag_u16(value: u16) -> i16 {
    ((value >> 1) as i16) ^ (-((value & 1) as i16))
}

/// Decodes a ZigZag mapped `u32`.
///
/// # Parameters
/// - `value`: ZigZag mapped unsigned value.
///
/// # Returns
/// Decoded signed value.
fn decode_zigzag_u32(value: u32) -> i32 {
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
}

/// Decodes a ZigZag mapped `u64`.
///
/// # Parameters
/// - `value`: ZigZag mapped unsigned value.
///
/// # Returns
/// Decoded signed value.
fn decode_zigzag_u64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

/// Decodes a ZigZag mapped `usize`.
///
/// # Parameters
/// - `value`: ZigZag mapped unsigned value.
///
/// # Returns
/// Decoded signed value.
fn decode_zigzag_usize(value: usize) -> isize {
    ((value >> 1) as isize) ^ (-((value & 1) as isize))
}
