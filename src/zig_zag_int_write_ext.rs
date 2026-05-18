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
    Result,
    Write,
};

use crate::Leb128IntWriteExt;

/// Extension methods for writing ZigZag encoded signed integers.
///
/// ZigZag maps signed integers to unsigned integers so small negative values
/// still have short varint encodings. The mapped unsigned value is written with
/// unsigned LEB128. The ZigZag mapping follows the Protocol Buffers encoding
/// guide:
/// <https://protobuf.dev/programming-guides/encoding/#signed-integers>.
pub trait ZigZagIntWriteExt: Write {
    /// Writes a ZigZag encoded `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_zigzag_i32(&mut self, value: i32) -> Result<()>;

    /// Writes a ZigZag encoded `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_zigzag_i64(&mut self, value: i64) -> Result<()>;

    /// Writes a ZigZag encoded `isize`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_zigzag_isize(&mut self, value: isize) -> Result<()>;
}

impl<T> ZigZagIntWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_zigzag_i32(&mut self, value: i32) -> Result<()> {
        self.write_uleb_u32(encode_zigzag_i32(value))
    }

    #[inline]
    fn write_zigzag_i64(&mut self, value: i64) -> Result<()> {
        self.write_uleb_u64(encode_zigzag_i64(value))
    }

    #[inline]
    fn write_zigzag_isize(&mut self, value: isize) -> Result<()> {
        self.write_uleb_usize(encode_zigzag_isize(value))
    }
}

/// Encodes an `i32` with ZigZag mapping.
///
/// # Parameters
/// - `value`: Signed value to map.
///
/// # Returns
/// ZigZag mapped unsigned value.
fn encode_zigzag_i32(value: i32) -> u32 {
    ((value as u32) << 1) ^ ((value >> 31) as u32)
}

/// Encodes an `i64` with ZigZag mapping.
///
/// # Parameters
/// - `value`: Signed value to map.
///
/// # Returns
/// ZigZag mapped unsigned value.
fn encode_zigzag_i64(value: i64) -> u64 {
    ((value as u64) << 1) ^ ((value >> 63) as u64)
}

/// Encodes an `isize` with ZigZag mapping.
///
/// # Parameters
/// - `value`: Signed value to map.
///
/// # Returns
/// ZigZag mapped unsigned value.
fn encode_zigzag_isize(value: isize) -> usize {
    ((value as usize) << 1) ^ ((value >> (isize::BITS - 1)) as usize)
}
