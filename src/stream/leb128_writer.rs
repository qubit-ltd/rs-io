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
    Seek,
    SeekFrom,
    Write,
};

use crate::codec::{
    Leb128Codec,
    NonStrict,
};
use crate::util::write_utf8_payload;

macro_rules! write_leb128_value {
    ($writer:expr, $value:expr, $ty:ty) => {
        write_leb128::<{ Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
            $writer,
            $value,
            |bytes, value| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length.
                unsafe { Leb128Codec::<$ty, NonStrict>::write_unchecked(bytes, 0, value) }
            },
        )
    };
}

/// Writer wrapper for canonical LEB128 integers.
pub struct Leb128Writer<W> {
    inner: W,
}

impl<W> Leb128Writer<W> {
    /// Creates a LEB128 writer.
    #[must_use]
    #[inline]
    pub const fn new(inner: W) -> Self {
        Self { inner }
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

impl<W> Leb128Writer<W>
where
    W: Write,
{
    /// Writes an unsigned LEB128 `u8`.
    #[inline]
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, u8)
    }

    /// Writes an unsigned LEB128 `u16`.
    #[inline]
    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, u16)
    }

    /// Writes an unsigned LEB128 `u32`.
    #[inline]
    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, u32)
    }

    /// Writes an unsigned LEB128 `u64`.
    #[inline]
    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, u64)
    }

    /// Writes an unsigned LEB128 `u128`.
    #[inline]
    pub fn write_u128(&mut self, value: u128) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, u128)
    }

    /// Writes an unsigned LEB128 `usize`.
    #[inline]
    pub fn write_usize(&mut self, value: usize) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, usize)
    }

    /// Writes a signed LEB128 `i8`.
    #[inline]
    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, i8)
    }

    /// Writes a signed LEB128 `i16`.
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, i16)
    }

    /// Writes a signed LEB128 `i32`.
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, i32)
    }

    /// Writes a signed LEB128 `i64`.
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, i64)
    }

    /// Writes a signed LEB128 `i128`.
    #[inline]
    pub fn write_i128(&mut self, value: i128) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, i128)
    }

    /// Writes a signed LEB128 `isize`.
    #[inline]
    pub fn write_isize(&mut self, value: isize) -> Result<()> {
        write_leb128_value!(&mut self.inner, value, isize)
    }

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
        write_utf8_payload(&mut self.inner, value)
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

#[inline]
fn write_leb128<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write,
    F: FnOnce(&mut [u8], T) -> usize,
{
    let mut bytes = [0u8; N];
    let len = encode(&mut bytes, value);
    writer.write_all(&bytes[..len])
}
