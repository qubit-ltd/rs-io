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

use crate::ZigZagReadExt;

/// Reader wrapper for ZigZag encoded signed integers.
///
/// This wrapper defaults to non-strict decoding of the underlying unsigned
/// LEB128 value. Use [`ZigZagReader::with_strict`] or
/// [`ZigZagReader::set_strict`] to reject non-canonical underlying encodings.
///
/// # Examples
/// ```
/// use std::io::Cursor;
///
/// use qubit_io::{
///     ZigZagReader,
///     ZigZagWriter,
/// };
///
/// let mut output = ZigZagWriter::new(Vec::new());
/// output.write_i32(-123)?;
///
/// let mut input = ZigZagReader::with_strict(Cursor::new(output.into_inner()), true);
/// assert_eq!(-123, input.read_i32()?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct ZigZagReader<R> {
    inner: R,
    strict: bool,
}

impl<R> ZigZagReader<R> {
    /// Creates a ZigZag reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    ///
    /// # Returns
    /// A new non-strict ZigZag reader.
    #[inline]
    pub fn new(inner: R) -> Self {
        Self::with_strict(inner, false)
    }

    /// Creates a ZigZag reader with explicit underlying LEB128 policy.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `strict`: Whether to reject non-canonical underlying LEB128 encodings.
    ///
    /// # Returns
    /// A new ZigZag reader.
    #[inline]
    pub fn with_strict(inner: R, strict: bool) -> Self {
        Self { inner, strict }
    }

    /// Reports whether strict underlying LEB128 decoding is enabled.
    ///
    /// # Returns
    /// `true` when non-canonical underlying LEB128 encodings are rejected.
    #[inline]
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// Changes the underlying LEB128 policy used by subsequent reads.
    ///
    /// # Parameters
    /// - `strict`: Whether to reject non-canonical underlying LEB128 encodings.
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
        #[doc = concat!("Reads a ZigZag encoded `", stringify!($value), "`.")]
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped reader, or `InvalidData` for
        /// malformed or overflowing underlying unsigned LEB128 input. In strict
        /// mode, also returns `InvalidData` for non-canonical underlying LEB128
        /// input.
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

impl<R> ZigZagReader<R>
where
    R: Read,
{
    delegate_read!(read_i8, read_zigzag_i8, read_zigzag_i8_strict, i8);
    delegate_read!(read_i16, read_zigzag_i16, read_zigzag_i16_strict, i16);
    delegate_read!(read_i32, read_zigzag_i32, read_zigzag_i32_strict, i32);
    delegate_read!(read_i64, read_zigzag_i64, read_zigzag_i64_strict, i64);
    delegate_read!(read_i128, read_zigzag_i128, read_zigzag_i128_strict, i128);
    delegate_read!(
        read_isize,
        read_zigzag_isize,
        read_zigzag_isize_strict,
        isize
    );
}

impl<R> Read for ZigZagReader<R>
where
    R: Read,
{
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}

impl<R> BufRead for ZigZagReader<R>
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

impl<R> Seek for ZigZagReader<R>
where
    R: Seek,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
