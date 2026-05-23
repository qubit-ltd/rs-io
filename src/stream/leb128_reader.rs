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
use std::io::{
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::codec::{
    DecodePolicy,
    Leb128Codec,
    NonStrict,
};
use crate::util::{
    read_leb128_payload,
    read_utf8_payload,
};

macro_rules! read_leb128_value {
    ($reader:expr, $ty:ty, $policy:ty) => {
        read_leb128_payload::<{ Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
            $reader,
            |bytes| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length,
                // or it contains an earlier terminating byte before decoding.
                unsafe { Leb128Codec::<$ty, $policy>::read_unchecked(bytes, 0) }
            },
        )
    };
}

/// Reader wrapper for LEB128 integers.
///
/// The decoding policy is selected by the `P` type parameter. Use
/// `Leb128Reader<R, NonStrict>` for permissive decoding and
/// `Leb128Reader<R, Strict>` for canonical-only decoding.
pub struct Leb128Reader<R, P = NonStrict> {
    inner: R,
    marker: PhantomData<fn() -> P>,
}

impl<R, P> Leb128Reader<R, P>
where
    P: DecodePolicy,
{
    /// Creates a LEB128 reader.
    #[must_use]
    #[inline]
    pub const fn new(inner: R) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Returns whether this reader rejects non-canonical encodings.
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

impl<R, P> Leb128Reader<R, P>
where
    R: Read,
    P: DecodePolicy,
{
    /// Reads an unsigned LEB128 `u8`.
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8> {
        read_leb128_value!(&mut self.inner, u8, P)
    }

    /// Reads an unsigned LEB128 `u16`.
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16> {
        read_leb128_value!(&mut self.inner, u16, P)
    }

    /// Reads an unsigned LEB128 `u32`.
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32> {
        read_leb128_value!(&mut self.inner, u32, P)
    }

    /// Reads an unsigned LEB128 `u64`.
    #[inline]
    pub fn read_u64(&mut self) -> Result<u64> {
        read_leb128_value!(&mut self.inner, u64, P)
    }

    /// Reads an unsigned LEB128 `u128`.
    #[inline]
    pub fn read_u128(&mut self) -> Result<u128> {
        read_leb128_value!(&mut self.inner, u128, P)
    }

    /// Reads an unsigned LEB128 `usize`.
    #[inline]
    pub fn read_usize(&mut self) -> Result<usize> {
        read_leb128_value!(&mut self.inner, usize, P)
    }

    /// Reads a signed LEB128 `i8`.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8> {
        read_leb128_value!(&mut self.inner, i8, P)
    }

    /// Reads a signed LEB128 `i16`.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16> {
        read_leb128_value!(&mut self.inner, i16, P)
    }

    /// Reads a signed LEB128 `i32`.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32> {
        read_leb128_value!(&mut self.inner, i32, P)
    }

    /// Reads a signed LEB128 `i64`.
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        read_leb128_value!(&mut self.inner, i64, P)
    }

    /// Reads a signed LEB128 `i128`.
    #[inline]
    pub fn read_i128(&mut self) -> Result<i128> {
        read_leb128_value!(&mut self.inner, i128, P)
    }

    /// Reads a signed LEB128 `isize`.
    #[inline]
    pub fn read_isize(&mut self) -> Result<isize> {
        read_leb128_value!(&mut self.inner, isize, P)
    }

    /// Reads a UTF-8 string prefixed by an unsigned LEB128 byte length.
    ///
    /// # Parameters
    ///
    /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
    ///
    /// # Returns
    ///
    /// Returns the decoded UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns an I/O error for length or payload reads, [`std::io::ErrorKind::InvalidData`]
    /// when the encoded length exceeds `max_len`, or [`std::io::ErrorKind::InvalidData`]
    /// when the payload is not valid UTF-8.
    #[inline]
    pub fn read_utf8_string(&mut self, max_len: usize) -> Result<String> {
        let len = self.read_usize()?;
        read_utf8_payload(&mut self.inner, len, max_len)
    }
}

impl<R, P> Read for Leb128Reader<R, P>
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

impl<R, P> Seek for Leb128Reader<R, P>
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
