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
};

use crate::codec::{
    DecodePolicy,
    NonStrict,
    ZigZagCodec,
};
use crate::util::read_leb128_payload;

macro_rules! read_zig_zag_value {
    ($reader:expr, $ty:ty, $policy:ty) => {
        read_leb128_payload::<{ ZigZagCodec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
            $reader,
            |bytes| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length,
                // or it contains an earlier terminating byte before decoding.
                unsafe { ZigZagCodec::<$ty, $policy>::read_unchecked(bytes, 0) }
            },
        )
    };
}

/// Reader wrapper for ZigZag + unsigned LEB128 integers.
pub struct ZigZagReader<R, P = NonStrict> {
    inner: R,
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
    /// Reads a ZigZag `i8`.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8> {
        read_zig_zag_value!(&mut self.inner, i8, P)
    }

    /// Reads a ZigZag `i16`.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16> {
        read_zig_zag_value!(&mut self.inner, i16, P)
    }

    /// Reads a ZigZag `i32`.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32> {
        read_zig_zag_value!(&mut self.inner, i32, P)
    }

    /// Reads a ZigZag `i64`.
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        read_zig_zag_value!(&mut self.inner, i64, P)
    }

    /// Reads a ZigZag `i128`.
    #[inline]
    pub fn read_i128(&mut self) -> Result<i128> {
        read_zig_zag_value!(&mut self.inner, i128, P)
    }

    /// Reads a ZigZag `isize`.
    #[inline]
    pub fn read_isize(&mut self) -> Result<isize> {
        read_zig_zag_value!(&mut self.inner, isize, P)
    }
}
