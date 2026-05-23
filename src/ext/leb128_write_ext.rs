/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Result, Write};

use crate::codec::{Leb128Codec, NonStrict};

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

/// Extension methods for writing canonical LEB128 integers to byte streams.
pub trait Leb128WriteExt: Write {
    /// Writes an unsigned LEB128 `u8`.
    #[inline]
    fn write_uleb_u8(&mut self, value: u8) -> Result<()> {
        write_leb128_value!(self, value, u8)
    }

    /// Writes an unsigned LEB128 `u16`.
    #[inline]
    fn write_uleb_u16(&mut self, value: u16) -> Result<()> {
        write_leb128_value!(self, value, u16)
    }

    /// Writes an unsigned LEB128 `u32`.
    #[inline]
    fn write_uleb_u32(&mut self, value: u32) -> Result<()> {
        write_leb128_value!(self, value, u32)
    }

    /// Writes an unsigned LEB128 `u64`.
    #[inline]
    fn write_uleb_u64(&mut self, value: u64) -> Result<()> {
        write_leb128_value!(self, value, u64)
    }

    /// Writes an unsigned LEB128 `u128`.
    #[inline]
    fn write_uleb_u128(&mut self, value: u128) -> Result<()> {
        write_leb128_value!(self, value, u128)
    }

    /// Writes an unsigned LEB128 `usize`.
    #[inline]
    fn write_uleb_usize(&mut self, value: usize) -> Result<()> {
        write_leb128_value!(self, value, usize)
    }

    /// Writes a signed LEB128 `i8`.
    #[inline]
    fn write_sleb_i8(&mut self, value: i8) -> Result<()> {
        write_leb128_value!(self, value, i8)
    }

    /// Writes a signed LEB128 `i16`.
    #[inline]
    fn write_sleb_i16(&mut self, value: i16) -> Result<()> {
        write_leb128_value!(self, value, i16)
    }

    /// Writes a signed LEB128 `i32`.
    #[inline]
    fn write_sleb_i32(&mut self, value: i32) -> Result<()> {
        write_leb128_value!(self, value, i32)
    }

    /// Writes a signed LEB128 `i64`.
    #[inline]
    fn write_sleb_i64(&mut self, value: i64) -> Result<()> {
        write_leb128_value!(self, value, i64)
    }

    /// Writes a signed LEB128 `i128`.
    #[inline]
    fn write_sleb_i128(&mut self, value: i128) -> Result<()> {
        write_leb128_value!(self, value, i128)
    }

    /// Writes a signed LEB128 `isize`.
    #[inline]
    fn write_sleb_isize(&mut self, value: isize) -> Result<()> {
        write_leb128_value!(self, value, isize)
    }
}

impl<W> Leb128WriteExt for W where W: Write + ?Sized {}

#[inline]
fn write_leb128<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut [u8], T) -> usize,
{
    let mut bytes = [0u8; N];
    let len = encode(&mut bytes, value);
    writer.write_all(&bytes[..len])
}
