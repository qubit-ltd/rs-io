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
    NonStrict,
    ZigZagCodec,
};

macro_rules! write_zig_zag_value {
    ($writer:expr, $value:expr, $ty:ty) => {
        write_zig_zag::<{ ZigZagCodec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
            $writer,
            $value,
            |bytes, value| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length.
                unsafe { ZigZagCodec::<$ty, NonStrict>::write_unchecked(bytes, 0, value) }
            },
        )
    };
}

/// Writer wrapper for canonical ZigZag + unsigned LEB128 integers.
pub struct ZigZagWriter<W> {
    inner: W,
}

impl<W> ZigZagWriter<W> {
    /// Creates a ZigZag writer.
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

impl<W> ZigZagWriter<W>
where
    W: Write,
{
    /// Writes a ZigZag `i8`.
    #[inline]
    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, i8)
    }

    /// Writes a ZigZag `i16`.
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, i16)
    }

    /// Writes a ZigZag `i32`.
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, i32)
    }

    /// Writes a ZigZag `i64`.
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, i64)
    }

    /// Writes a ZigZag `i128`.
    #[inline]
    pub fn write_i128(&mut self, value: i128) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, i128)
    }

    /// Writes a ZigZag `isize`.
    #[inline]
    pub fn write_isize(&mut self, value: isize) -> Result<()> {
        write_zig_zag_value!(&mut self.inner, value, isize)
    }
}

impl<W> Write for ZigZagWriter<W>
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

impl<W> Seek for ZigZagWriter<W>
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
fn write_zig_zag<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write,
    F: FnOnce(&mut [u8], T) -> usize,
{
    let mut bytes = [0u8; N];
    let len = encode(&mut bytes, value);
    writer.write_all(&bytes[..len])
}
