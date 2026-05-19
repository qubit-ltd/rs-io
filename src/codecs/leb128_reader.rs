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
    Leb128ReadExt,
    StringReadExt,
};

/// Reader wrapper for LEB128 integers and LEB128 length-prefixed strings.
///
/// # Examples
/// ```
/// use std::io::Cursor;
///
/// use qubit_io::{
///     Leb128Reader,
///     Leb128Writer,
/// };
///
/// let mut output = Leb128Writer::new(Vec::new());
/// output.write_uleb_u16(300)?;
///
/// let mut input = Leb128Reader::new(Cursor::new(output.into_inner()));
/// assert_eq!(300, input.read_uleb_u16()?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct Leb128Reader<R> {
    inner: R,
}

impl<R> Leb128Reader<R> {
    /// Creates a LEB128 reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    ///
    /// # Returns
    /// A new LEB128 reader.
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Returns an immutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

macro_rules! delegate_read {
    ($name:ident, $value:ty) => {
        #[doc = concat!("Reads a LEB128 `", stringify!($value), "`.")]
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped reader, or `InvalidData` for
        /// malformed or overflowing LEB128 input.
        pub fn $name(&mut self) -> Result<$value> {
            self.inner.$name()
        }
    };
}

impl<R> Leb128Reader<R>
where
    R: Read,
{
    delegate_read!(read_uleb_u8, u8);
    delegate_read!(read_uleb_u8_strict, u8);
    delegate_read!(read_uleb_u16, u16);
    delegate_read!(read_uleb_u16_strict, u16);
    delegate_read!(read_uleb_u32, u32);
    delegate_read!(read_uleb_u32_strict, u32);
    delegate_read!(read_uleb_u64, u64);
    delegate_read!(read_uleb_u64_strict, u64);
    delegate_read!(read_uleb_u128, u128);
    delegate_read!(read_uleb_u128_strict, u128);
    delegate_read!(read_uleb_usize, usize);
    delegate_read!(read_uleb_usize_strict, usize);
    delegate_read!(read_sleb_i8, i8);
    delegate_read!(read_sleb_i8_strict, i8);
    delegate_read!(read_sleb_i16, i16);
    delegate_read!(read_sleb_i16_strict, i16);
    delegate_read!(read_sleb_i32, i32);
    delegate_read!(read_sleb_i32_strict, i32);
    delegate_read!(read_sleb_i64, i64);
    delegate_read!(read_sleb_i64_strict, i64);
    delegate_read!(read_sleb_i128, i128);
    delegate_read!(read_sleb_i128_strict, i128);
    delegate_read!(read_sleb_isize, isize);
    delegate_read!(read_sleb_isize_strict, isize);

    /// Reads a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader, or `InvalidData` when the
    /// encoded length exceeds `max_len` or the payload is not valid UTF-8.
    pub fn read_utf8_string(&mut self, max_len: usize) -> Result<String> {
        self.inner.read_utf8_string_uleb(max_len)
    }

    /// Reads a UTF-8 string with a canonical unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader, or `InvalidData` when the
    /// length prefix is malformed or non-canonical, the encoded length exceeds
    /// `max_len`, or the payload is not valid UTF-8.
    pub fn read_utf8_string_strict(&mut self, max_len: usize) -> Result<String> {
        self.inner.read_utf8_string_uleb_strict(max_len)
    }
}

impl<R> Read for Leb128Reader<R>
where
    R: Read,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}
