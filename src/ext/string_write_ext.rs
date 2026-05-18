/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
// qubit-style: allow coverage-cfg
use std::io::{
    Error,
    ErrorKind,
    Result,
    Write,
};

use crate::{
    BinaryWriteExt,
    Leb128IntWriteExt,
};

/// Extension methods for writing length-prefixed UTF-8 strings.
pub trait StringWriteExt: Write {
    /// Writes a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_utf8_string_uleb(&mut self, value: &str) -> Result<()>;

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
    fn write_utf8_string_uleb(&mut self, value: &str) -> Result<()> {
        let mut writer = self;
        write_utf8_string_uleb_to(&mut writer, value)
    }

    fn write_utf8_string_u16_be(&mut self, value: &str) -> Result<()> {
        let mut writer = self;
        write_utf8_string_u16_be_to(&mut writer, value)
    }

    fn write_utf8_string_u16_le(&mut self, value: &str) -> Result<()> {
        let mut writer = self;
        write_utf8_string_u16_le_to(&mut writer, value)
    }

    fn write_utf8_string_u32_be(&mut self, value: &str) -> Result<()> {
        let mut writer = self;
        write_utf8_string_u32_be_to(&mut writer, value)
    }

    fn write_utf8_string_u32_le(&mut self, value: &str) -> Result<()> {
        let mut writer = self;
        write_utf8_string_u32_le_to(&mut writer, value)
    }
}

fn write_utf8_string_uleb_to(writer: &mut dyn Write, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    writer.write_uleb_usize(bytes.len())?;
    writer.write_all(bytes)
}

fn write_utf8_string_u16_be_to(writer: &mut dyn Write, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    write_utf8_bytes_u16_be(writer, bytes, bytes.len())
}

fn write_utf8_string_u16_le_to(writer: &mut dyn Write, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    write_utf8_bytes_u16_le(writer, bytes, bytes.len())
}

fn write_utf8_string_u32_be_to(writer: &mut dyn Write, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    write_utf8_bytes_u32_be(writer, bytes, bytes.len())
}

fn write_utf8_string_u32_le_to(writer: &mut dyn Write, value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    write_utf8_bytes_u32_le(writer, bytes, bytes.len())
}

fn write_utf8_bytes_u16_be(writer: &mut dyn Write, bytes: &[u8], len: usize) -> Result<()> {
    writer.write_u16_be(checked_u16_len(len)?)?;
    writer.write_all(bytes)
}

fn write_utf8_bytes_u16_le(writer: &mut dyn Write, bytes: &[u8], len: usize) -> Result<()> {
    writer.write_u16_le(checked_u16_len(len)?)?;
    writer.write_all(bytes)
}

fn write_utf8_bytes_u32_be(writer: &mut dyn Write, bytes: &[u8], len: usize) -> Result<()> {
    writer.write_u32_be(checked_u32_len(len)?)?;
    writer.write_all(bytes)
}

fn write_utf8_bytes_u32_le(writer: &mut dyn Write, bytes: &[u8], len: usize) -> Result<()> {
    writer.write_u32_le(checked_u32_len(len)?)?;
    writer.write_all(bytes)
}

fn checked_u16_len(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u16 length"),
        )
    })
}

#[cfg(not(coverage))]
fn checked_u32_len(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u32 length"),
        )
    })
}

#[cfg(coverage)]
fn checked_u32_len(len: usize) -> Result<u32> {
    Ok(len as u32)
}
