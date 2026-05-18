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

/// Extension methods for writing LEB128 encoded integers.
///
/// Unsigned methods write unsigned LEB128 values, and signed methods write
/// signed LEB128 values. Both forms encode seven payload bits per byte in
/// least-significant group first order, with the high bit marking
/// continuation. The integer encoding is described by the WebAssembly Core
/// binary format:
/// <https://webassembly.github.io/spec/core/binary/values.html#integers>.
pub trait Leb128IntWriteExt: Write {
    /// Writes an unsigned LEB128 `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uleb_u32(&mut self, value: u32) -> Result<()>;

    /// Writes an unsigned LEB128 `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uleb_u64(&mut self, value: u64) -> Result<()>;

    /// Writes an unsigned LEB128 `usize`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uleb_usize(&mut self, value: usize) -> Result<()>;

    /// Writes a signed LEB128 `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_sleb_i32(&mut self, value: i32) -> Result<()>;

    /// Writes a signed LEB128 `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_sleb_i64(&mut self, value: i64) -> Result<()>;

    /// Writes a signed LEB128 `isize`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_sleb_isize(&mut self, value: isize) -> Result<()>;
}

impl<T> Leb128IntWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_uleb_u32(&mut self, value: u32) -> Result<()> {
        write_uleb(self, value as u64)
    }

    #[inline]
    fn write_uleb_u64(&mut self, value: u64) -> Result<()> {
        write_uleb(self, value)
    }

    #[inline]
    fn write_uleb_usize(&mut self, value: usize) -> Result<()> {
        write_uleb(self, value as u64)
    }

    #[inline]
    fn write_sleb_i32(&mut self, value: i32) -> Result<()> {
        write_sleb(self, value as i64)
    }

    #[inline]
    fn write_sleb_i64(&mut self, value: i64) -> Result<()> {
        write_sleb(self, value)
    }

    #[inline]
    fn write_sleb_isize(&mut self, value: isize) -> Result<()> {
        write_sleb(self, value as i64)
    }
}

/// Writes an unsigned LEB128 integer.
///
/// # Parameters
/// - `writer`: Destination writer. It may be a sized writer or a writer trait
///   object.
/// - `value`: Value to encode.
///
/// # Errors
/// Returns an I/O error from `writer`.
fn write_uleb<T>(writer: &mut T, value: u64) -> Result<()>
where
    T: Write + ?Sized,
{
    let mut remaining = value;
    while remaining > 0x7f {
        writer.write_all(&[((remaining as u8) & 0x7f) | 0x80])?;
        remaining >>= 7;
    }
    writer.write_all(&[remaining as u8])
}

/// Writes a signed LEB128 integer.
///
/// # Parameters
/// - `writer`: Destination writer. It may be a sized writer or a writer trait
///   object.
/// - `value`: Value to encode.
///
/// # Errors
/// Returns an I/O error from `writer`.
fn write_sleb<T>(writer: &mut T, value: i64) -> Result<()>
where
    T: Write + ?Sized,
{
    let mut remaining = value;
    loop {
        let byte = (remaining as u8) & 0x7f;
        remaining >>= 7;
        let is_done = (remaining == 0 && byte & 0x40 == 0) || (remaining == -1 && byte & 0x40 != 0);
        if is_done {
            return writer.write_all(&[byte]);
        }
        writer.write_all(&[byte | 0x80])?;
    }
}
