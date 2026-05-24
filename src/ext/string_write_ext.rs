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

use crate::util::{
    write_utf8_payload as write_utf8_payload_impl,
    write_utf8_string_with_u16_len,
    write_utf8_string_with_u32_len,
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
    /// The length prefix is encoded as `usize`, so this format is target-width
    /// dependent. Prefer `u16` or `u32` length-prefix methods for persistent
    /// files and cross-platform protocols.
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
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16(&mut self, value: &str, byte_order: ByteOrder) -> Result<()>;

    /// Writes a UTF-8 string with a big-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16_be(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a little-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u16`, or an I/O error from the underlying writer.
    fn write_utf8_string_u16_le(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a runtime-order `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    /// - `byte_order`: Byte order used by the length prefix.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32(&mut self, value: &str, byte_order: ByteOrder) -> Result<()>;

    /// Writes a UTF-8 string with a big-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32_be(&mut self, value: &str) -> Result<()>;

    /// Writes a UTF-8 string with a little-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidInput`] when the UTF-8 byte length does not
    /// fit into `u32`, or an I/O error from the underlying writer.
    fn write_utf8_string_u32_le(&mut self, value: &str) -> Result<()>;
}

impl<T> StringWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_utf8_payload(&mut self, value: &str) -> Result<()> {
        write_utf8_payload_impl(self, value)
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
