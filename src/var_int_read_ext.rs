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

/// Extension methods for reading unsigned LEB128-style variable-length integers.
///
/// Values are encoded seven bits per byte, least-significant group first. The
/// high bit marks continuation. Malformed encodings that cannot fit the target
/// type are reported as [`ErrorKind::InvalidData`].
pub trait VarIntReadExt: Read {
    /// Reads a variable-length `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uvar_u32(&mut self) -> Result<u32>;

    /// Reads a variable-length `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uvar_u64(&mut self) -> Result<u64>;

    /// Reads a variable-length `usize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uvar_usize(&mut self) -> Result<usize>;
}

impl<T> VarIntReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_uvar_u32(&mut self) -> Result<u32> {
        read_uvar(self, u32::BITS, "u32").map(|value| value as u32)
    }

    #[inline]
    fn read_uvar_u64(&mut self) -> Result<u64> {
        read_uvar(self, u64::BITS, "u64")
    }

    #[inline]
    fn read_uvar_usize(&mut self) -> Result<usize> {
        read_uvar(self, usize::BITS, "usize").map(|value| value as usize)
    }
}

/// Reads an unsigned variable-length integer constrained to `bits`.
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
fn read_uvar<T>(reader: &mut T, bits: u32, type_name: &'static str) -> Result<u64>
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
            return Err(invalid_varint(type_name));
        }
        value |= (payload as u64) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(invalid_varint(type_name))
}

/// Builds an invalid-data error for malformed variable-length integers.
///
/// # Parameters
/// - `type_name`: Target type name.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error.
fn invalid_varint(type_name: &'static str) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("malformed variable-length integer for {type_name}"),
    )
}
