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

use crate::{
    BinaryReadExt,
    ByteOrder,
    StringReadExt,
};

/// Reader wrapper for binary scalar and length-prefixed string decoding.
///
/// Multi-byte methods use the configured [`ByteOrder`].
///
/// # Examples
/// ```
/// use std::io::Cursor;
///
/// use qubit_io::{
///     BinaryReader,
///     BinaryWriter,
///     ByteOrder,
/// };
///
/// let mut output = BinaryWriter::new(Vec::new(), ByteOrder::BigEndian);
/// output.write_u16(0x1234)?;
///
/// let mut input = BinaryReader::new(Cursor::new(output.into_inner()), ByteOrder::BigEndian);
/// assert_eq!(0x1234, input.read_u16()?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct BinaryReader<R> {
    inner: R,
    byte_order: ByteOrder,
}

impl<R> BinaryReader<R> {
    /// Creates a binary reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `byte_order`: Byte order used by multi-byte reads.
    ///
    /// # Returns
    /// A new binary reader.
    #[inline]
    pub fn new(inner: R, byte_order: ByteOrder) -> Self {
        Self { inner, byte_order }
    }

    /// Returns an immutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline]
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the byte order used by multi-byte reads.
    ///
    /// # Returns
    /// The configured byte order.
    #[inline]
    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    /// Changes the byte order used by multi-byte reads.
    ///
    /// # Parameters
    /// - `byte_order`: New byte order.
    #[inline]
    pub fn set_byte_order(&mut self, byte_order: ByteOrder) {
        self.byte_order = byte_order;
    }
}

impl<R> BinaryReader<R>
where
    R: Read,
{
    /// Reads one unsigned byte.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8> {
        self.inner.read_u8()
    }

    /// Reads one signed byte.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8> {
        self.inner.read_i8()
    }

    /// Reads a `u16` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16> {
        self.inner.read_u16(self.byte_order)
    }

    /// Reads an `i16` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16> {
        self.inner.read_i16(self.byte_order)
    }

    /// Reads a `u32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32> {
        self.inner.read_u32(self.byte_order)
    }

    /// Reads an `i32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32> {
        self.inner.read_i32(self.byte_order)
    }

    /// Reads a `u64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_u64(&mut self) -> Result<u64> {
        self.inner.read_u64(self.byte_order)
    }

    /// Reads an `i64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        self.inner.read_i64(self.byte_order)
    }

    /// Reads a `u128` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_u128(&mut self) -> Result<u128> {
        self.inner.read_u128(self.byte_order)
    }

    /// Reads an `i128` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_i128(&mut self) -> Result<i128> {
        self.inner.read_i128(self.byte_order)
    }

    /// Reads an `f32` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_f32(&mut self) -> Result<f32> {
        self.inner.read_f32(self.byte_order)
    }

    /// Reads an `f64` using the configured byte order.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader.
    #[inline]
    pub fn read_f64(&mut self) -> Result<f64> {
        self.inner.read_f64(self.byte_order)
    }

    /// Reads a UTF-8 string with a `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader, or `InvalidData` when the
    /// payload length exceeds `max_len` or the payload is not valid UTF-8.
    #[inline]
    pub fn read_utf8_string_u16(&mut self, max_len: usize) -> Result<String> {
        match self.byte_order {
            ByteOrder::BigEndian => self.inner.read_utf8_string_u16_be(max_len),
            ByteOrder::LittleEndian => self.inner.read_utf8_string_u16_le(max_len),
        }
    }

    /// Reads a UTF-8 string with a `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader, or `InvalidData` when the
    /// payload length exceeds `max_len` or the payload is not valid UTF-8.
    #[inline]
    pub fn read_utf8_string_u32(&mut self, max_len: usize) -> Result<String> {
        match self.byte_order {
            ByteOrder::BigEndian => self.inner.read_utf8_string_u32_be(max_len),
            ByteOrder::LittleEndian => self.inner.read_utf8_string_u32_le(max_len),
        }
    }
}

impl<R> Read for BinaryReader<R>
where
    R: Read,
{
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}
