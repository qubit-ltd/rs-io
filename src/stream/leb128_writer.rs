/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Result, Seek, SeekFrom, Write};

use crate::codec::{Leb128Codec, NonStrict};
use crate::WriteExt;

/// Writer wrapper for canonical LEB128 integers.
pub struct Leb128Writer<W> {
    inner: W,
    buffer: [u8; 19],
}

impl<W> Leb128Writer<W> {
    /// Creates a LEB128 writer.
    #[must_use]
    #[inline]
    pub const fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: [0; 19],
        }
    }

    /// Returns a shared reference to the underlying writer.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns an exclusive reference to the underlying writer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the underlying writer.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

macro_rules! impl_write_value {
    ($method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self, value: $ty) -> Result<()> {
            type Codec = Leb128Codec<$ty, NonStrict>;

            self.write_leb128::<$ty, { Codec::REQUIRED_MIN_BUFFER_LEN }, _>(
                value,
                |bytes, value| unsafe { Codec::write_unchecked(bytes, 0, value) },
            )
        }
    };
}

impl<W> Leb128Writer<W>
where
    W: Write,
{
    #[inline]
    fn write_leb128<T, const N: usize, F>(&mut self, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8; 19], T) -> usize,
    {
        let len = encode(&mut self.buffer, value);
        // SAFETY: The codec returns a length within the fixed internal buffer.
        unsafe { self.inner.write_all_unchecked(&self.buffer, 0, len) }
    }

    impl_write_value!(write_u8, u8, "Writes an unsigned LEB128 `u8`.");
    impl_write_value!(write_u16, u16, "Writes an unsigned LEB128 `u16`.");
    impl_write_value!(write_u32, u32, "Writes an unsigned LEB128 `u32`.");
    impl_write_value!(write_u64, u64, "Writes an unsigned LEB128 `u64`.");
    impl_write_value!(write_u128, u128, "Writes an unsigned LEB128 `u128`.");
    impl_write_value!(write_usize, usize, "Writes an unsigned LEB128 `usize`.");
    impl_write_value!(write_i8, i8, "Writes a signed LEB128 `i8`.");
    impl_write_value!(write_i16, i16, "Writes a signed LEB128 `i16`.");
    impl_write_value!(write_i32, i32, "Writes a signed LEB128 `i32`.");
    impl_write_value!(write_i64, i64, "Writes a signed LEB128 `i64`.");
    impl_write_value!(write_i128, i128, "Writes a signed LEB128 `i128`.");
    impl_write_value!(write_isize, isize, "Writes a signed LEB128 `isize`.");

    /// Writes a UTF-8 string prefixed by an unsigned LEB128 byte length.
    ///
    /// # Parameters
    ///
    /// - `value`: String slice to write.
    ///
    /// # Errors
    ///
    /// Returns an I/O error from the underlying writer.
    #[inline]
    pub fn write_utf8_string(&mut self, value: &str) -> Result<()> {
        self.write_usize(value.len())?;
        let bytes = value.as_bytes();
        // SAFETY: The range covers the full byte slice produced by `str::as_bytes`.
        unsafe { self.inner.write_all_unchecked(bytes, 0, bytes.len()) }
    }
}

impl<W> Write for Leb128Writer<W>
where
    W: Write,
{
    /// Writes bytes to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Source bytes to write.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped writer.
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.inner.write(buffer)
    }

    /// Flushes the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped writer.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W> Seek for Leb128Writer<W>
where
    W: Seek,
{
    /// Seeks the wrapped writer.
    ///
    /// # Parameters
    ///
    /// - `position`: Target seek position.
    ///
    /// # Returns
    ///
    /// Returns the new stream position.
    ///
    /// # Errors
    ///
    /// Returns the seek error reported by the wrapped writer.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
