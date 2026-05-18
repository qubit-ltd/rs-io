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

/// Extension methods for writing binary scalar values to [`Write`] streams.
///
/// The methods use explicit byte-order suffixes for multi-byte values and
/// delegate to [`Write::write_all`], so they either write the complete encoded
/// value or return the first I/O error.
pub trait BinaryWriteExt: Write {
    /// Writes one unsigned byte.
    ///
    /// # Parameters
    /// - `value`: Byte to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u8(&mut self, value: u8) -> Result<()>;

    /// Writes one signed byte.
    ///
    /// # Parameters
    /// - `value`: Byte to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i8(&mut self, value: i8) -> Result<()>;

    /// Writes a big-endian `u16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u16_be(&mut self, value: u16) -> Result<()>;

    /// Writes a little-endian `u16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u16_le(&mut self, value: u16) -> Result<()>;

    /// Writes a big-endian `i16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i16_be(&mut self, value: i16) -> Result<()>;

    /// Writes a little-endian `i16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i16_le(&mut self, value: i16) -> Result<()>;

    /// Writes a big-endian `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u32_be(&mut self, value: u32) -> Result<()>;

    /// Writes a little-endian `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u32_le(&mut self, value: u32) -> Result<()>;

    /// Writes a big-endian `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i32_be(&mut self, value: i32) -> Result<()>;

    /// Writes a little-endian `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i32_le(&mut self, value: i32) -> Result<()>;

    /// Writes a big-endian `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u64_be(&mut self, value: u64) -> Result<()>;

    /// Writes a little-endian `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u64_le(&mut self, value: u64) -> Result<()>;

    /// Writes a big-endian `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i64_be(&mut self, value: i64) -> Result<()>;

    /// Writes a little-endian `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i64_le(&mut self, value: i64) -> Result<()>;

    /// Writes a big-endian IEEE-754 `f32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f32_be(&mut self, value: f32) -> Result<()>;

    /// Writes a little-endian IEEE-754 `f32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f32_le(&mut self, value: f32) -> Result<()>;

    /// Writes a big-endian IEEE-754 `f64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f64_be(&mut self, value: f64) -> Result<()>;

    /// Writes a little-endian IEEE-754 `f64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f64_le(&mut self, value: f64) -> Result<()>;
}

impl<T> BinaryWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write_all(&[value])
    }

    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_all(&[value as u8])
    }

    #[inline]
    fn write_u16_be(&mut self, value: u16) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u16_le(&mut self, value: u16) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i16_be(&mut self, value: i16) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i16_le(&mut self, value: i16) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_u32_be(&mut self, value: u32) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u32_le(&mut self, value: u32) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i32_be(&mut self, value: i32) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i32_le(&mut self, value: i32) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_u64_be(&mut self, value: u64) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u64_le(&mut self, value: u64) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i64_be(&mut self, value: i64) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i64_le(&mut self, value: i64) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_f32_be(&mut self, value: f32) -> Result<()> {
        self.write_u32_be(value.to_bits())
    }

    #[inline]
    fn write_f32_le(&mut self, value: f32) -> Result<()> {
        self.write_u32_le(value.to_bits())
    }

    #[inline]
    fn write_f64_be(&mut self, value: f64) -> Result<()> {
        self.write_u64_be(value.to_bits())
    }

    #[inline]
    fn write_f64_le(&mut self, value: f64) -> Result<()> {
        self.write_u64_le(value.to_bits())
    }
}
