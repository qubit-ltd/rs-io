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

use crate::util::read_utf8_payload as read_utf8_payload_impl;
use crate::{
    BinaryReadExt,
    ByteOrder,
    Leb128ReadExt,
};

/// Extension methods for reading length-prefixed UTF-8 strings.
pub trait StringReadExt: Read {
    /// Reads a UTF-8 payload with an already decoded byte length.
    ///
    /// # Parameters
    /// - `len`: UTF-8 payload length in bytes.
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for payload reads, [`std::io::ErrorKind::InvalidData`] when
    /// `len` exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`] when the payload
    /// is not valid UTF-8.
    fn read_utf8_payload(&mut self, len: usize, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// The length prefix is decoded as `usize`, so this format is target-width
    /// dependent. Prefer `u16` or `u32` length-prefix methods for persistent
    /// files and cross-platform protocols.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_uleb(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a canonical unsigned LEB128 byte-length prefix.
    ///
    /// The length prefix is decoded as `usize`, so this format is target-width
    /// dependent. Prefer `u16` or `u32` length-prefix methods for persistent
    /// files and cross-platform protocols.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the length prefix is malformed or non-canonical, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_uleb_strict(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a runtime-order `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `byte_order`: Byte order used by the length prefix.
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u16(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a big-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u16_be(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a little-endian `u16` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u16_le(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a runtime-order `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `byte_order`: Byte order used by the length prefix.
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u32(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a big-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u32_be(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a little-endian `u32` byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u32_le(&mut self, max_len: usize) -> Result<String>;
}

impl<T> StringReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_utf8_payload(&mut self, len: usize, max_len: usize) -> Result<String> {
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_uleb(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_uleb_usize()?;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_uleb_strict(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_uleb_usize_strict()?;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String> {
        let len = usize::from(self.read_u16(byte_order)?);
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16_be(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u16_be()? as usize;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16_le(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u16_le()? as usize;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String> {
        let len = self.read_u32(byte_order)? as usize;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32_be(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u32_be()? as usize;
        read_utf8_payload_impl(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32_le(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u32_le()? as usize;
        read_utf8_payload_impl(self, len, max_len)
    }
}
