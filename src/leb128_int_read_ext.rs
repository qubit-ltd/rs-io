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
    Error,
    ErrorKind,
    Read,
    Result,
};

use crate::BinaryReadExt;

/// Extension methods for reading LEB128 encoded integers.
///
/// Unsigned methods read unsigned LEB128 values, and signed methods read signed
/// LEB128 values. Both forms encode seven payload bits per byte in
/// least-significant group first order, with the high bit marking
/// continuation. The integer encoding is described by the WebAssembly Core
/// binary format:
/// <https://webassembly.github.io/spec/core/binary/values.html#integers>.
pub trait Leb128IntReadExt: Read {
    /// Reads an unsigned LEB128 `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u32(&mut self) -> Result<u32>;

    /// Reads an unsigned LEB128 `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u64(&mut self) -> Result<u64>;

    /// Reads an unsigned LEB128 `usize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_usize(&mut self) -> Result<usize>;

    /// Reads a signed LEB128 `i32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i32(&mut self) -> Result<i32>;

    /// Reads a signed LEB128 `i64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i64(&mut self) -> Result<i64>;

    /// Reads a signed LEB128 `isize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_isize(&mut self) -> Result<isize>;
}

impl<T> Leb128IntReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_uleb_u32(&mut self) -> Result<u32> {
        read_uleb(self, u32::BITS, "u32").map(|value| value as u32)
    }

    #[inline]
    fn read_uleb_u64(&mut self) -> Result<u64> {
        read_uleb(self, u64::BITS, "u64")
    }

    #[inline]
    fn read_uleb_usize(&mut self) -> Result<usize> {
        read_uleb(self, usize::BITS, "usize").map(|value| value as usize)
    }

    #[inline]
    fn read_sleb_i32(&mut self) -> Result<i32> {
        read_sleb(self, i32::BITS, "i32").map(|value| value as i32)
    }

    #[inline]
    fn read_sleb_i64(&mut self) -> Result<i64> {
        read_sleb(self, i64::BITS, "i64")
    }

    #[inline]
    fn read_sleb_isize(&mut self) -> Result<isize> {
        read_sleb(self, isize::BITS, "isize").map(|value| value as isize)
    }
}

/// Reads an unsigned LEB128 integer constrained to `bits`.
///
/// # Parameters
/// - `reader`: Source reader. It may be a sized reader or a reader trait
///   object.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
///
/// # Returns
/// Decoded value as `u64`.
///
/// # Errors
/// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed or
/// overflowing input, or another I/O error from `reader`.
fn read_uleb<T>(reader: &mut T, bits: u32, type_name: &'static str) -> Result<u64>
where
    T: Read + ?Sized,
{
    let max_bytes = bits.div_ceil(7);
    let remainder = bits % 7;
    let max_last_payload = ((1u16 << remainder) - 1) as u8;

    let mut value = 0u64;
    for index in 0..max_bytes {
        let byte = reader.read_u8()?;
        let payload = byte & 0x7f;
        let is_too_wide_final_byte = (index == max_bytes - 1) & (payload > max_last_payload);
        if is_too_wide_final_byte {
            return Err(invalid_leb128(type_name));
        }
        value |= (payload as u64) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(invalid_leb128(type_name))
}

/// Reads a signed LEB128 integer constrained to `bits`.
///
/// # Parameters
/// - `reader`: Source reader. It may be a sized reader or a reader trait
///   object.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
///
/// # Returns
/// Decoded value as `i64`.
///
/// # Errors
/// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed or
/// overflowing input, or another I/O error from `reader`.
fn read_sleb<T>(reader: &mut T, bits: u32, type_name: &'static str) -> Result<i64>
where
    T: Read + ?Sized,
{
    let max_bytes = bits.div_ceil(7);
    let mut value = 0i128;
    let mut shift = 0u32;
    for _ in 0..max_bytes {
        let byte = reader.read_u8()?;
        let payload = (byte & 0x7f) as i128;
        value |= payload << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if byte & 0x40 != 0 {
                value |= (!0i128) << shift;
            }
            if !fits_signed_width(value, bits) {
                return Err(invalid_leb128(type_name));
            }
            return Ok(value as i64);
        }
    }
    Err(invalid_leb128(type_name))
}

/// Returns whether `value` fits in a signed integer with `bits` bits.
///
/// # Parameters
/// - `value`: Value to check.
/// - `bits`: Signed integer width in bits.
///
/// # Returns
/// `true` when `value` can be represented by the signed width.
fn fits_signed_width(value: i128, bits: u32) -> bool {
    let min = -(1i128 << (bits - 1));
    let max = (1i128 << (bits - 1)) - 1;
    (min..=max).contains(&value)
}

/// Builds an invalid-data error for malformed LEB128 integers.
///
/// # Parameters
/// - `type_name`: Target type name.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error.
fn invalid_leb128(type_name: &'static str) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("malformed LEB128 integer for {type_name}"),
    )
}
