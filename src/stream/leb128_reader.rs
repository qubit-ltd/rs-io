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
    BufRead,
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::{
    Leb128ReadExt,
    StringReadExt,
};

/// Reader wrapper for LEB128 integers and LEB128 length-prefixed strings.
///
/// This wrapper defaults to non-strict decoding. Use [`Leb128Reader::with_strict`]
/// or [`Leb128Reader::set_strict`] to reject non-canonical LEB128 encodings.
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
/// output.write_u16(300)?;
///
/// let mut input = Leb128Reader::with_strict(Cursor::new(output.into_inner()), true);
/// assert_eq!(300, input.read_u16()?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct Leb128Reader<R> {
    inner: R,
    strict: bool,
}

impl<R> Leb128Reader<R> {
    /// Creates a LEB128 reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    ///
    /// # Returns
    /// A new non-strict LEB128 reader.
    #[inline]
    pub fn new(inner: R) -> Self {
        Self::with_strict(inner, false)
    }

    /// Creates a LEB128 reader with explicit canonical-decoding policy.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `strict`: Whether to reject non-canonical LEB128 encodings.
    ///
    /// # Returns
    /// A new LEB128 reader.
    #[inline]
    pub fn with_strict(inner: R, strict: bool) -> Self {
        Self { inner, strict }
    }

    /// Reports whether strict canonical LEB128 decoding is enabled.
    ///
    /// # Returns
    /// `true` when non-canonical encodings are rejected.
    #[inline]
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// Changes the canonical-decoding policy used by subsequent reads.
    ///
    /// # Parameters
    /// - `strict`: Whether to reject non-canonical LEB128 encodings.
    #[inline]
    pub fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
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
}

macro_rules! delegate_read {
    ($name:ident, $read:ident, $read_strict:ident, $value:ty) => {
        #[doc = concat!("Reads a LEB128 `", stringify!($value), "`.")]
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped reader, or `InvalidData` for
        /// malformed or overflowing LEB128 input. In strict mode, also returns
        /// `InvalidData` for non-canonical LEB128 input.
        #[inline]
        pub fn $name(&mut self) -> Result<$value> {
            if self.strict {
                self.inner.$read_strict()
            } else {
                self.inner.$read()
            }
        }
    };
}

impl<R> Leb128Reader<R>
where
    R: Read,
{
    delegate_read!(read_u8, read_uleb_u8, read_uleb_u8_strict, u8);
    delegate_read!(read_u16, read_uleb_u16, read_uleb_u16_strict, u16);
    delegate_read!(read_u32, read_uleb_u32, read_uleb_u32_strict, u32);
    delegate_read!(read_u64, read_uleb_u64, read_uleb_u64_strict, u64);
    delegate_read!(read_u128, read_uleb_u128, read_uleb_u128_strict, u128);
    delegate_read!(read_usize, read_uleb_usize, read_uleb_usize_strict, usize);
    delegate_read!(read_i8, read_sleb_i8, read_sleb_i8_strict, i8);
    delegate_read!(read_i16, read_sleb_i16, read_sleb_i16_strict, i16);
    delegate_read!(read_i32, read_sleb_i32, read_sleb_i32_strict, i32);
    delegate_read!(read_i64, read_sleb_i64, read_sleb_i64_strict, i64);
    delegate_read!(read_i128, read_sleb_i128, read_sleb_i128_strict, i128);
    delegate_read!(read_isize, read_sleb_isize, read_sleb_isize_strict, isize);

    /// Reads a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped reader, or `InvalidData` when the
    /// encoded length exceeds `max_len` or the payload is not valid UTF-8. In
    /// strict mode, also returns `InvalidData` when the length prefix is
    /// non-canonical.
    #[inline]
    pub fn read_utf8_string(&mut self, max_len: usize) -> Result<String> {
        if self.strict {
            self.inner.read_utf8_string_uleb_strict(max_len)
        } else {
            self.inner.read_utf8_string_uleb(max_len)
        }
    }
}

impl<R> Read for Leb128Reader<R>
where
    R: Read,
{
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}

impl<R> BufRead for Leb128Reader<R>
where
    R: BufRead,
{
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_buf()
    }

    #[inline]
    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

impl<R> Seek for Leb128Reader<R>
where
    R: Seek,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
