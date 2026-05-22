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

/// Extension methods for writing ZigZag + unsigned LEB128 integers.
pub trait ZigZagWriteExt: Write {
    /// Writes a ZigZag `i8`.
    #[inline]
    fn write_zig_zag_i8(&mut self, value: i8) -> Result<()> {
        write_zig_zag_value!(self, value, i8)
    }

    /// Writes a ZigZag `i16`.
    #[inline]
    fn write_zig_zag_i16(&mut self, value: i16) -> Result<()> {
        write_zig_zag_value!(self, value, i16)
    }

    /// Writes a ZigZag `i32`.
    #[inline]
    fn write_zig_zag_i32(&mut self, value: i32) -> Result<()> {
        write_zig_zag_value!(self, value, i32)
    }

    /// Writes a ZigZag `i64`.
    #[inline]
    fn write_zig_zag_i64(&mut self, value: i64) -> Result<()> {
        write_zig_zag_value!(self, value, i64)
    }

    /// Writes a ZigZag `i128`.
    #[inline]
    fn write_zig_zag_i128(&mut self, value: i128) -> Result<()> {
        write_zig_zag_value!(self, value, i128)
    }

    /// Writes a ZigZag `isize`.
    #[inline]
    fn write_zig_zag_isize(&mut self, value: isize) -> Result<()> {
        write_zig_zag_value!(self, value, isize)
    }
}

impl<W> ZigZagWriteExt for W where W: Write + ?Sized {}

#[inline]
fn write_zig_zag<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut [u8], T) -> usize,
{
    let mut bytes = [0u8; N];
    let len = encode(&mut bytes, value);
    writer.write_all(&bytes[..len])
}
