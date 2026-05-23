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
    Result,
    Write,
};

use crate::{
    BinaryWriteExt,
    ByteOrder,
    Leb128WriteExt,
};

/// Extension methods for writing length-prefixed UTF-8 strings.
pub trait StringWriteExt: Write {
    /// Writes a UTF-8 payload without a length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_utf8_payload(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_utf8_string_uleb(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a runtime-order `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    /// - `byte_order`: Byte order used by the length prefix.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16(&mut self, value: &str, byte_order: ByteOrder) -> Result<()>;

    /// Writes a UTF-8 string with a big-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16_be(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a little-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16_le(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a runtime-order `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    /// - `byte_order`: Byte order used by the length prefix.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32(&mut self, value: &str, byte_order: ByteOrder) -> Result<()>;

    /// Writes a UTF-8 string with a big-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32_be(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a little-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32_le(&mut self, value: &str) -> Result<()>;
}

impl<T> StringWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_utf8_payload(&mut self, value: &str) -> Result<()> {
        self.write_all(value.as_bytes())
    }

    #[inline]
    fn write_utf8_string_uleb(&mut self, value: &str) -> Result<()> {
        let bytes = value.as_bytes();
        self.write_uleb_usize(bytes.len())?;
        self.write_all(bytes)
    }

    #[inline]
    fn write_utf8_string_u16(&mut self, value: &str, byte_order: ByteOrder) -> Result<()> {
        write_utf8_string_with_u16_len(self, value, |writer, len| writer.write_u16(len, byte_order))
    }

    #[inline]
    fn write_utf8_string_u16_be(&mut self, value: &str) -> Result<()> {
        write_utf8_string_with_u16_len(self, value, |writer, len| writer.write_u16_be(len))
    }

    #[inline]
    fn write_utf8_string_u16_le(&mut self, value: &str) -> Result<()> {
        write_utf8_string_with_u16_len(self, value, |writer, len| writer.write_u16_le(len))
    }

    #[inline]
    fn write_utf8_string_u32(&mut self, value: &str, byte_order: ByteOrder) -> Result<()> {
        write_utf8_string_with_u32_len(self, value, |writer, len| writer.write_u32(len, byte_order))
    }

    #[inline]
    fn write_utf8_string_u32_be(&mut self, value: &str) -> Result<()> {
        write_utf8_string_with_u32_len(self, value, |writer, len| writer.write_u32_be(len))
    }

    #[inline]
    fn write_utf8_string_u32_le(&mut self, value: &str) -> Result<()> {
        write_utf8_string_with_u32_len(self, value, |writer, len| writer.write_u32_le(len))
    }
}

/// Writes a UTF-8 string after a `u16` byte-length prefix.
///
/// # Parameters
/// - `writer`: Destination writer.
/// - `value`: String slice to write.
/// - `write_len`: Callback that writes the encoded `u16` length.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not fit
/// into `u16`, or an I/O error from the underlying writer.
fn write_utf8_string_with_u16_len<W, F>(writer: &mut W, value: &str, write_len: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut W, u16) -> Result<()>,
{
    let bytes = value.as_bytes();
    write_len(writer, checked_u16_len(bytes.len())?)?;
    writer.write_all(bytes)
}

/// Writes a UTF-8 string after a `u32` byte-length prefix.
///
/// # Parameters
/// - `writer`: Destination writer.
/// - `value`: String slice to write.
/// - `write_len`: Callback that writes the encoded `u32` length.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not fit
/// into `u32`, or an I/O error from the underlying writer.
fn write_utf8_string_with_u32_len<W, F>(writer: &mut W, value: &str, write_len: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut W, u32) -> Result<()>,
{
    let bytes = value.as_bytes();
    write_len(writer, checked_u32_len(bytes.len())?)?;
    writer.write_all(bytes)
}

/// Converts a UTF-8 payload length to a `u16` length prefix value.
///
/// # Parameters
/// - `len`: Payload length in bytes.
///
/// # Returns
/// The payload length represented as `u16`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `len` is larger than `u16::MAX`.
fn checked_u16_len(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u16 length"),
        )
    })
}

/// Converts a UTF-8 payload length to a `u32` length prefix value.
///
/// # Parameters
/// - `len`: Payload length in bytes.
///
/// # Returns
/// The payload length represented as `u32`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `len` is larger than `u32::MAX`.
fn checked_u32_len(len: usize) -> Result<u32> {
    if len > u32::MAX as usize {
        Err(Error::new(ErrorKind::InvalidInput, "string is too long"))
    } else {
        Ok(len as u32)
    }
}
