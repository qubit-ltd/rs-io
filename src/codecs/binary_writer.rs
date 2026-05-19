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

use crate::{
    BinaryWriteExt,
    ByteOrder,
    StringWriteExt,
};

/// Writer wrapper for binary scalar and length-prefixed string encoding.
///
/// Multi-byte methods use the configured [`ByteOrder`].
///
/// # Examples
/// ```
/// use qubit_io::{
///     BinaryWriter,
///     ByteOrder,
/// };
///
/// let mut output = BinaryWriter::new(Vec::new(), ByteOrder::LittleEndian);
/// output.write_u16(0x1234)?;
///
/// assert_eq!(vec![0x34, 0x12], output.into_inner());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct BinaryWriter<W> {
    inner: W,
    byte_order: ByteOrder,
}

impl<W> BinaryWriter<W> {
    /// Creates a binary writer.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    /// - `byte_order`: Byte order used by multi-byte writes.
    ///
    /// # Returns
    /// A new binary writer.
    #[inline]
    pub fn new(inner: W, byte_order: ByteOrder) -> Self {
        Self { inner, byte_order }
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Returns the byte order used by multi-byte writes.
    ///
    /// # Returns
    /// The configured byte order.
    #[inline]
    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    /// Changes the byte order used by multi-byte writes.
    ///
    /// # Parameters
    /// - `byte_order`: New byte order.
    #[inline]
    pub fn set_byte_order(&mut self, byte_order: ByteOrder) {
        self.byte_order = byte_order;
    }
}

impl<W> BinaryWriter<W>
where
    W: Write,
{
    /// Writes one unsigned byte.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.inner.write_u8(value)
    }

    /// Writes one signed byte.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        self.inner.write_i8(value)
    }

    /// Writes a `u16` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        self.inner.write_u16(value, self.byte_order)
    }

    /// Writes an `i16` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        self.inner.write_i16(value, self.byte_order)
    }

    /// Writes a `u32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.inner.write_u32(value, self.byte_order)
    }

    /// Writes an `i32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.inner.write_i32(value, self.byte_order)
    }

    /// Writes a `u64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        self.inner.write_u64(value, self.byte_order)
    }

    /// Writes an `i64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        self.inner.write_i64(value, self.byte_order)
    }

    /// Writes a `u128` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_u128(&mut self, value: u128) -> Result<()> {
        self.inner.write_u128(value, self.byte_order)
    }

    /// Writes an `i128` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_i128(&mut self, value: i128) -> Result<()> {
        self.inner.write_i128(value, self.byte_order)
    }

    /// Writes an `f32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_f32(&mut self, value: f32) -> Result<()> {
        self.inner.write_f32(value, self.byte_order)
    }

    /// Writes an `f64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    #[inline]
    pub fn write_f64(&mut self, value: f64) -> Result<()> {
        self.inner.write_f64(value, self.byte_order)
    }

    /// Writes a UTF-8 string with a `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer, or `InvalidInput` when
    /// the UTF-8 byte length does not fit into `u16`.
    #[inline]
    pub fn write_utf8_string_u16(&mut self, value: &str) -> Result<()> {
        match self.byte_order {
            ByteOrder::BigEndian => self.inner.write_utf8_string_u16_be(value),
            ByteOrder::LittleEndian => self.inner.write_utf8_string_u16_le(value),
        }
    }

    /// Writes a UTF-8 string with a `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer, or `InvalidInput` when
    /// the UTF-8 byte length does not fit into `u32`.
    #[inline]
    pub fn write_utf8_string_u32(&mut self, value: &str) -> Result<()> {
        match self.byte_order {
            ByteOrder::BigEndian => self.inner.write_utf8_string_u32_be(value),
            ByteOrder::LittleEndian => self.inner.write_utf8_string_u32_le(value),
        }
    }
}

impl<W> Write for BinaryWriter<W>
where
    W: Write,
{
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.inner.write(buffer)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
