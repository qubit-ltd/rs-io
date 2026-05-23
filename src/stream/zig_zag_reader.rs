/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use core::marker::PhantomData;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};

use crate::ReadExt;
use crate::codec::{DecodePolicy, Leb128DecodeError, NonStrict};
use crate::stream::macros::read_zig_zag_value;

/// Reader wrapper for ZigZag + unsigned LEB128 integers.
pub struct ZigZagReader<R, P = NonStrict> {
    inner: R,
    buffer: [u8; 19],
    marker: PhantomData<fn() -> P>,
}

impl<R, P> ZigZagReader<R, P>
where
    P: DecodePolicy,
{
    /// Creates a ZigZag reader.
    #[must_use]
    #[inline]
    pub const fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: [0; 19],
            marker: PhantomData,
        }
    }

    /// Returns whether this reader rejects non-canonical LEB128 encodings.
    #[must_use]
    #[inline]
    pub const fn is_strict(&self) -> bool {
        P::STRICT
    }

    /// Returns a shared reference to the underlying reader.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns an exclusive reference to the underlying reader.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the underlying reader.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R, P> ZigZagReader<R, P>
where
    R: Read,
    P: DecodePolicy,
{
    #[inline]
    fn read_leb128<T, const N: usize, F>(&mut self, decode: F) -> Result<T>
    where
        F: FnOnce(&[u8; 19]) -> std::result::Result<(T, usize), Leb128DecodeError>,
    {
        debug_assert!(
            N <= self.buffer.len(),
            "ZigZag read length exceeds internal buffer"
        );
        for index in 0..N {
            // SAFETY: `index` is produced by `0..N`, where `N` is a
            // codec-declared length that fits the fixed internal buffer.
            unsafe {
                self.inner
                    .read_exact_unchecked(&mut self.buffer, index, 1)?;
            }
            if read_byte(&self.buffer, index) & 0x80 == 0 {
                return decode(&self.buffer)
                    .map(|(value, _)| value)
                    .map_err(map_leb128_decode_error);
            }
        }
        decode(&self.buffer)
            .map(|(value, _)| value)
            .map_err(map_leb128_decode_error)
    }

    /// Reads a ZigZag `i8`.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8> {
        read_zig_zag_value!(self, i8, P)
    }

    /// Reads a ZigZag `i16`.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16> {
        read_zig_zag_value!(self, i16, P)
    }

    /// Reads a ZigZag `i32`.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32> {
        read_zig_zag_value!(self, i32, P)
    }

    /// Reads a ZigZag `i64`.
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        read_zig_zag_value!(self, i64, P)
    }

    /// Reads a ZigZag `i128`.
    #[inline]
    pub fn read_i128(&mut self) -> Result<i128> {
        read_zig_zag_value!(self, i128, P)
    }

    /// Reads a ZigZag `isize`.
    #[inline]
    pub fn read_isize(&mut self) -> Result<isize> {
        read_zig_zag_value!(self, isize, P)
    }
}

impl<R, P> Read for ZigZagReader<R, P>
where
    R: Read,
{
    /// Reads bytes from the wrapped reader.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Destination byte buffer.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped reader.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}

impl<R, P> Seek for ZigZagReader<R, P>
where
    R: Seek,
{
    /// Seeks the wrapped reader.
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
    /// Returns the seek error reported by the wrapped reader.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}

#[inline]
fn map_leb128_decode_error(error: Leb128DecodeError) -> Error {
    Error::new(ErrorKind::InvalidData, error)
}

/// Reads one byte from the internal ZigZag buffer without an extra bounds check.
#[inline(always)]
fn read_byte(buffer: &[u8; 19], index: usize) -> u8 {
    debug_assert!(
        index < buffer.len(),
        "ZigZag read index exceeds internal buffer"
    );
    // SAFETY: `read_leb128` only calls this with an index produced by
    // `0..N`, where N is a codec-declared length that fits `buffer`.
    unsafe { *buffer.as_ptr().add(index) }
}
