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

/// Extension methods for writing unsigned LEB128-style variable-length integers.
///
/// Values are encoded seven bits per byte, least-significant group first. The
/// high bit marks continuation.
pub trait VarIntWriteExt: Write {
    /// Writes a variable-length `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uvar_u32(&mut self, value: u32) -> Result<()>;

    /// Writes a variable-length `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uvar_u64(&mut self, value: u64) -> Result<()>;

    /// Writes a variable-length `usize`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_uvar_usize(&mut self, value: usize) -> Result<()>;
}

impl<T> VarIntWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_uvar_u32(&mut self, value: u32) -> Result<()> {
        write_uvar(self, value as u64)
    }

    #[inline]
    fn write_uvar_u64(&mut self, value: u64) -> Result<()> {
        write_uvar(self, value)
    }

    #[inline]
    fn write_uvar_usize(&mut self, value: usize) -> Result<()> {
        write_uvar(self, value as u64)
    }
}

/// Writes an unsigned variable-length integer.
///
/// # Parameters
/// - `writer`: Destination writer. It may be a sized writer or a writer trait
///   object.
/// - `value`: Value to encode.
///
/// # Errors
/// Returns an I/O error from `writer`.
fn write_uvar<T>(writer: &mut T, value: u64) -> Result<()>
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
