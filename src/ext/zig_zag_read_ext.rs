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

use crate::codec::{
    NonStrict,
    Strict,
    ZigZagCodec,
};
use crate::util::read_leb128_payload;

macro_rules! read_zig_zag_value {
    ($reader:expr, $ty:ty, $policy:ty) => {
        read_leb128_payload::<{ ZigZagCodec::<$ty, $policy>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>($reader, |bytes| {
            // SAFETY: The local buffer is exactly the codec's minimum buffer length,
            // or it contains an earlier terminating byte before decoding.
            unsafe { ZigZagCodec::<$ty, $policy>::read_unchecked(bytes, 0) }
        })
    };
}

/// Extension methods for reading ZigZag + unsigned LEB128 integers.
pub trait ZigZagReadExt: Read {
    /// Reads a non-strict ZigZag `i8`.
    #[inline]
    fn read_zig_zag_i8(&mut self) -> Result<i8> {
        read_zig_zag_value!(self, i8, NonStrict)
    }

    /// Reads a strict ZigZag `i8`.
    #[inline]
    fn read_zig_zag_i8_strict(&mut self) -> Result<i8> {
        read_zig_zag_value!(self, i8, Strict)
    }

    /// Reads a non-strict ZigZag `i16`.
    #[inline]
    fn read_zig_zag_i16(&mut self) -> Result<i16> {
        read_zig_zag_value!(self, i16, NonStrict)
    }

    /// Reads a strict ZigZag `i16`.
    #[inline]
    fn read_zig_zag_i16_strict(&mut self) -> Result<i16> {
        read_zig_zag_value!(self, i16, Strict)
    }

    /// Reads a non-strict ZigZag `i32`.
    #[inline]
    fn read_zig_zag_i32(&mut self) -> Result<i32> {
        read_zig_zag_value!(self, i32, NonStrict)
    }

    /// Reads a strict ZigZag `i32`.
    #[inline]
    fn read_zig_zag_i32_strict(&mut self) -> Result<i32> {
        read_zig_zag_value!(self, i32, Strict)
    }

    /// Reads a non-strict ZigZag `i64`.
    #[inline]
    fn read_zig_zag_i64(&mut self) -> Result<i64> {
        read_zig_zag_value!(self, i64, NonStrict)
    }

    /// Reads a strict ZigZag `i64`.
    #[inline]
    fn read_zig_zag_i64_strict(&mut self) -> Result<i64> {
        read_zig_zag_value!(self, i64, Strict)
    }

    /// Reads a non-strict ZigZag `i128`.
    #[inline]
    fn read_zig_zag_i128(&mut self) -> Result<i128> {
        read_zig_zag_value!(self, i128, NonStrict)
    }

    /// Reads a strict ZigZag `i128`.
    #[inline]
    fn read_zig_zag_i128_strict(&mut self) -> Result<i128> {
        read_zig_zag_value!(self, i128, Strict)
    }

    /// Reads a non-strict ZigZag `isize`.
    #[inline]
    fn read_zig_zag_isize(&mut self) -> Result<isize> {
        read_zig_zag_value!(self, isize, NonStrict)
    }

    /// Reads a strict ZigZag `isize`.
    #[inline]
    fn read_zig_zag_isize_strict(&mut self) -> Result<isize> {
        read_zig_zag_value!(self, isize, Strict)
    }
}

impl<R> ZigZagReadExt for R where R: Read + ?Sized {}
