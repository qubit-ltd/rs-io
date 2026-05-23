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
    Leb128Codec,
    NonStrict,
    Strict,
};
use crate::util::read_leb128_payload;

macro_rules! read_leb128_value {
    ($reader:expr, $ty:ty, $policy:ty) => {
        read_leb128_payload::<{ Leb128Codec::<$ty, $policy>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>($reader, |bytes| {
            // SAFETY: The local buffer is exactly the codec's minimum buffer length,
            // or it contains an earlier terminating byte before decoding.
            unsafe { Leb128Codec::<$ty, $policy>::read_unchecked(bytes, 0) }
        })
    };
}

/// Extension methods for reading LEB128 integers from byte streams.
pub trait Leb128ReadExt: Read {
    /// Reads a non-strict unsigned LEB128 `u8`.
    #[inline]
    fn read_uleb_u8(&mut self) -> Result<u8> {
        read_leb128_value!(self, u8, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `u8`.
    #[inline]
    fn read_uleb_u8_strict(&mut self) -> Result<u8> {
        read_leb128_value!(self, u8, Strict)
    }

    /// Reads a non-strict unsigned LEB128 `u16`.
    #[inline]
    fn read_uleb_u16(&mut self) -> Result<u16> {
        read_leb128_value!(self, u16, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `u16`.
    #[inline]
    fn read_uleb_u16_strict(&mut self) -> Result<u16> {
        read_leb128_value!(self, u16, Strict)
    }

    /// Reads a non-strict unsigned LEB128 `u32`.
    #[inline]
    fn read_uleb_u32(&mut self) -> Result<u32> {
        read_leb128_value!(self, u32, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `u32`.
    #[inline]
    fn read_uleb_u32_strict(&mut self) -> Result<u32> {
        read_leb128_value!(self, u32, Strict)
    }

    /// Reads a non-strict unsigned LEB128 `u64`.
    #[inline]
    fn read_uleb_u64(&mut self) -> Result<u64> {
        read_leb128_value!(self, u64, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `u64`.
    #[inline]
    fn read_uleb_u64_strict(&mut self) -> Result<u64> {
        read_leb128_value!(self, u64, Strict)
    }

    /// Reads a non-strict unsigned LEB128 `u128`.
    #[inline]
    fn read_uleb_u128(&mut self) -> Result<u128> {
        read_leb128_value!(self, u128, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `u128`.
    #[inline]
    fn read_uleb_u128_strict(&mut self) -> Result<u128> {
        read_leb128_value!(self, u128, Strict)
    }

    /// Reads a non-strict unsigned LEB128 `usize`.
    #[inline]
    fn read_uleb_usize(&mut self) -> Result<usize> {
        read_leb128_value!(self, usize, NonStrict)
    }

    /// Reads a strict unsigned LEB128 `usize`.
    #[inline]
    fn read_uleb_usize_strict(&mut self) -> Result<usize> {
        read_leb128_value!(self, usize, Strict)
    }

    /// Reads a non-strict signed LEB128 `i8`.
    #[inline]
    fn read_sleb_i8(&mut self) -> Result<i8> {
        read_leb128_value!(self, i8, NonStrict)
    }

    /// Reads a strict signed LEB128 `i8`.
    #[inline]
    fn read_sleb_i8_strict(&mut self) -> Result<i8> {
        read_leb128_value!(self, i8, Strict)
    }

    /// Reads a non-strict signed LEB128 `i16`.
    #[inline]
    fn read_sleb_i16(&mut self) -> Result<i16> {
        read_leb128_value!(self, i16, NonStrict)
    }

    /// Reads a strict signed LEB128 `i16`.
    #[inline]
    fn read_sleb_i16_strict(&mut self) -> Result<i16> {
        read_leb128_value!(self, i16, Strict)
    }

    /// Reads a non-strict signed LEB128 `i32`.
    #[inline]
    fn read_sleb_i32(&mut self) -> Result<i32> {
        read_leb128_value!(self, i32, NonStrict)
    }

    /// Reads a strict signed LEB128 `i32`.
    #[inline]
    fn read_sleb_i32_strict(&mut self) -> Result<i32> {
        read_leb128_value!(self, i32, Strict)
    }

    /// Reads a non-strict signed LEB128 `i64`.
    #[inline]
    fn read_sleb_i64(&mut self) -> Result<i64> {
        read_leb128_value!(self, i64, NonStrict)
    }

    /// Reads a strict signed LEB128 `i64`.
    #[inline]
    fn read_sleb_i64_strict(&mut self) -> Result<i64> {
        read_leb128_value!(self, i64, Strict)
    }

    /// Reads a non-strict signed LEB128 `i128`.
    #[inline]
    fn read_sleb_i128(&mut self) -> Result<i128> {
        read_leb128_value!(self, i128, NonStrict)
    }

    /// Reads a strict signed LEB128 `i128`.
    #[inline]
    fn read_sleb_i128_strict(&mut self) -> Result<i128> {
        read_leb128_value!(self, i128, Strict)
    }

    /// Reads a non-strict signed LEB128 `isize`.
    #[inline]
    fn read_sleb_isize(&mut self) -> Result<isize> {
        read_leb128_value!(self, isize, NonStrict)
    }

    /// Reads a strict signed LEB128 `isize`.
    #[inline]
    fn read_sleb_isize_strict(&mut self) -> Result<isize> {
        read_leb128_value!(self, isize, Strict)
    }
}

impl<R> Leb128ReadExt for R where R: Read + ?Sized {}
