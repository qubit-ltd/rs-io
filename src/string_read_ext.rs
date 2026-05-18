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

use crate::{
    BinaryReadExt,
    VarIntReadExt,
};

/// Extension methods for reading length-prefixed UTF-8 strings.
pub trait StringReadExt: Read {
    /// Reads a UTF-8 string with an unsigned varint byte-length prefix.
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
    fn read_utf8_string_uvar(&mut self, max_len: usize) -> Result<String>;

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
    T: Read,
{
    fn read_utf8_string_uvar(&mut self, max_len: usize) -> Result<String> {
        read_utf8_string_uvar_impl(self, max_len)
    }

    fn read_utf8_string_u32_be(&mut self, max_len: usize) -> Result<String> {
        read_utf8_string_u32_be_impl(self, max_len)
    }

    fn read_utf8_string_u32_le(&mut self, max_len: usize) -> Result<String> {
        read_utf8_string_u32_le_impl(self, max_len)
    }
}

fn read_utf8_string_uvar_impl(reader: &mut dyn Read, max_len: usize) -> Result<String> {
    let len = reader.read_uvar_usize()?;
    read_utf8_payload(reader, len, max_len)
}

fn read_utf8_string_u32_be_impl(reader: &mut dyn Read, max_len: usize) -> Result<String> {
    let len = reader.read_u32_be()? as usize;
    read_utf8_payload(reader, len, max_len)
}

fn read_utf8_string_u32_le_impl(reader: &mut dyn Read, max_len: usize) -> Result<String> {
    let len = reader.read_u32_le()? as usize;
    read_utf8_payload(reader, len, max_len)
}

fn read_utf8_payload(reader: &mut dyn Read, len: usize, max_len: usize) -> Result<String> {
    if len > max_len {
        return Err(length_exceeded_error(len, max_len));
    }
    let mut bytes = vec![0; len];
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
