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
use std::string::FromUtf8Error;

use crate::util::try_reserve_vec;
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
    /// Returns an I/O error for payload reads, [`ErrorKind::InvalidData`] when
    /// `len` exceeds `max_len`, or [`ErrorKind::InvalidData`] when the payload
    /// is not valid UTF-8.
    fn read_utf8_payload(&mut self, len: usize, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_uleb(&mut self, max_len: usize) -> Result<String>;

    /// Reads a UTF-8 string with a canonical unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    /// The decoded string.
    ///
    /// # Errors
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the length prefix is malformed or non-canonical, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
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
    /// Returns an I/O error for length or payload reads, [`ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    fn read_utf8_string_u32_le(&mut self, max_len: usize) -> Result<String>;
}

impl<T> StringReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_utf8_payload(&mut self, len: usize, max_len: usize) -> Result<String> {
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_uleb(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_uleb_usize()?;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_uleb_strict(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_uleb_usize_strict()?;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String> {
        let len = usize::from(self.read_u16(byte_order)?);
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16_be(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u16_be()? as usize;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u16_le(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u16_le()? as usize;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32(&mut self, byte_order: ByteOrder, max_len: usize) -> Result<String> {
        let len = self.read_u32(byte_order)? as usize;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32_be(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u32_be()? as usize;
        read_utf8_payload(self, len, max_len)
    }

    #[inline]
    fn read_utf8_string_u32_le(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_u32_le()? as usize;
        read_utf8_payload(self, len, max_len)
    }
}

/// Reads a UTF-8 payload after its length has already been decoded.
///
/// # Parameters
/// - `reader`: Reader that provides the UTF-8 payload bytes.
/// - `len`: Payload length in bytes.
/// - `max_len`: Maximum accepted payload length in bytes.
///
/// # Returns
/// Returns the decoded UTF-8 string.
///
/// # Errors
/// Returns an error when `len` exceeds `max_len`, allocation fails, the reader
/// cannot provide enough bytes, or the payload is not valid UTF-8.
fn read_utf8_payload<T>(reader: &mut T, len: usize, max_len: usize) -> Result<String>
where
    T: Read + ?Sized,
{
    if len > max_len {
        return Err(length_exceeded_error(len, max_len));
    }
    let mut bytes = Vec::new();
    try_reserve_vec(&mut bytes, len)?;
    bytes.resize(len, 0);
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(invalid_utf8_error)
}

fn length_exceeded_error(len: usize, max_len: usize) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("string length {len} exceeds maximum length of {max_len} bytes"),
    )
}

fn invalid_utf8_error(error: FromUtf8Error) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("length-prefixed string is not valid UTF-8: {error}"),
    )
}
