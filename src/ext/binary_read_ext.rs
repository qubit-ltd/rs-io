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
    BigEndian,
    BinaryCodec,
    ByteOrder,
    LittleEndian,
};

macro_rules! read_binary_value {
    ($reader:expr, $ty:ty, $order:ty) => {
        read_binary::<{ BinaryCodec::<$ty, $order>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
            $reader,
            |bytes| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length.
                unsafe { BinaryCodec::<$ty, $order>::read_unchecked(bytes, 0) }
            },
        )
    };
}

/// Extension methods for reading fixed-width binary values from byte streams.
pub trait BinaryReadExt: Read {
    /// Reads exactly `N` bytes.
    ///
    /// # Returns
    ///
    /// Returns the filled byte array.
    ///
    /// # Errors
    ///
    /// Returns any I/O error reported by the underlying reader.
    #[inline]
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut bytes = [0u8; N];
        self.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    /// Reads an unsigned 8-bit integer.
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        read_binary_value!(self, u8, BigEndian)
    }

    /// Reads a signed 8-bit integer.
    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        read_binary_value!(self, i8, BigEndian)
    }

    /// Reads an unsigned 16-bit integer using a runtime byte order.
    #[inline]
    fn read_u16(&mut self, byte_order: ByteOrder) -> Result<u16> {
        match byte_order {
            ByteOrder::BigEndian => self.read_u16_be(),
            ByteOrder::LittleEndian => self.read_u16_le(),
        }
    }

    /// Reads a big-endian unsigned 16-bit integer.
    #[inline]
    fn read_u16_be(&mut self) -> Result<u16> {
        read_binary_value!(self, u16, BigEndian)
    }

    /// Reads a little-endian unsigned 16-bit integer.
    #[inline]
    fn read_u16_le(&mut self) -> Result<u16> {
        read_binary_value!(self, u16, LittleEndian)
    }

    /// Reads an unsigned 32-bit integer using a runtime byte order.
    #[inline]
    fn read_u32(&mut self, byte_order: ByteOrder) -> Result<u32> {
        match byte_order {
            ByteOrder::BigEndian => self.read_u32_be(),
            ByteOrder::LittleEndian => self.read_u32_le(),
        }
    }

    /// Reads a big-endian unsigned 32-bit integer.
    #[inline]
    fn read_u32_be(&mut self) -> Result<u32> {
        read_binary_value!(self, u32, BigEndian)
    }

    /// Reads a little-endian unsigned 32-bit integer.
    #[inline]
    fn read_u32_le(&mut self) -> Result<u32> {
        read_binary_value!(self, u32, LittleEndian)
    }

    /// Reads an unsigned 64-bit integer using a runtime byte order.
    #[inline]
    fn read_u64(&mut self, byte_order: ByteOrder) -> Result<u64> {
        match byte_order {
            ByteOrder::BigEndian => self.read_u64_be(),
            ByteOrder::LittleEndian => self.read_u64_le(),
        }
    }

    /// Reads a big-endian unsigned 64-bit integer.
    #[inline]
    fn read_u64_be(&mut self) -> Result<u64> {
        read_binary_value!(self, u64, BigEndian)
    }

    /// Reads a little-endian unsigned 64-bit integer.
    #[inline]
    fn read_u64_le(&mut self) -> Result<u64> {
        read_binary_value!(self, u64, LittleEndian)
    }

    /// Reads an unsigned 128-bit integer using a runtime byte order.
    #[inline]
    fn read_u128(&mut self, byte_order: ByteOrder) -> Result<u128> {
        match byte_order {
            ByteOrder::BigEndian => self.read_u128_be(),
            ByteOrder::LittleEndian => self.read_u128_le(),
        }
    }

    /// Reads a big-endian unsigned 128-bit integer.
    #[inline]
    fn read_u128_be(&mut self) -> Result<u128> {
        read_binary_value!(self, u128, BigEndian)
    }

    /// Reads a little-endian unsigned 128-bit integer.
    #[inline]
    fn read_u128_le(&mut self) -> Result<u128> {
        read_binary_value!(self, u128, LittleEndian)
    }

    /// Reads a signed 16-bit integer using a runtime byte order.
    #[inline]
    fn read_i16(&mut self, byte_order: ByteOrder) -> Result<i16> {
        match byte_order {
            ByteOrder::BigEndian => self.read_i16_be(),
            ByteOrder::LittleEndian => self.read_i16_le(),
        }
    }

    /// Reads a big-endian signed 16-bit integer.
    #[inline]
    fn read_i16_be(&mut self) -> Result<i16> {
        read_binary_value!(self, i16, BigEndian)
    }

    /// Reads a little-endian signed 16-bit integer.
    #[inline]
    fn read_i16_le(&mut self) -> Result<i16> {
        read_binary_value!(self, i16, LittleEndian)
    }

    /// Reads a signed 32-bit integer using a runtime byte order.
    #[inline]
    fn read_i32(&mut self, byte_order: ByteOrder) -> Result<i32> {
        match byte_order {
            ByteOrder::BigEndian => self.read_i32_be(),
            ByteOrder::LittleEndian => self.read_i32_le(),
        }
    }

    /// Reads a big-endian signed 32-bit integer.
    #[inline]
    fn read_i32_be(&mut self) -> Result<i32> {
        read_binary_value!(self, i32, BigEndian)
    }

    /// Reads a little-endian signed 32-bit integer.
    #[inline]
    fn read_i32_le(&mut self) -> Result<i32> {
        read_binary_value!(self, i32, LittleEndian)
    }

    /// Reads a signed 64-bit integer using a runtime byte order.
    #[inline]
    fn read_i64(&mut self, byte_order: ByteOrder) -> Result<i64> {
        match byte_order {
            ByteOrder::BigEndian => self.read_i64_be(),
            ByteOrder::LittleEndian => self.read_i64_le(),
        }
    }

    /// Reads a big-endian signed 64-bit integer.
    #[inline]
    fn read_i64_be(&mut self) -> Result<i64> {
        read_binary_value!(self, i64, BigEndian)
    }

    /// Reads a little-endian signed 64-bit integer.
    #[inline]
    fn read_i64_le(&mut self) -> Result<i64> {
        read_binary_value!(self, i64, LittleEndian)
    }

    /// Reads a signed 128-bit integer using a runtime byte order.
    #[inline]
    fn read_i128(&mut self, byte_order: ByteOrder) -> Result<i128> {
        match byte_order {
            ByteOrder::BigEndian => self.read_i128_be(),
            ByteOrder::LittleEndian => self.read_i128_le(),
        }
    }

    /// Reads a big-endian signed 128-bit integer.
    #[inline]
    fn read_i128_be(&mut self) -> Result<i128> {
        read_binary_value!(self, i128, BigEndian)
    }

    /// Reads a little-endian signed 128-bit integer.
    #[inline]
    fn read_i128_le(&mut self) -> Result<i128> {
        read_binary_value!(self, i128, LittleEndian)
    }

    /// Reads a 32-bit float using a runtime byte order.
    #[inline]
    fn read_f32(&mut self, byte_order: ByteOrder) -> Result<f32> {
        match byte_order {
            ByteOrder::BigEndian => self.read_f32_be(),
            ByteOrder::LittleEndian => self.read_f32_le(),
        }
    }

    /// Reads a big-endian 32-bit float.
    #[inline]
    fn read_f32_be(&mut self) -> Result<f32> {
        read_binary_value!(self, f32, BigEndian)
    }

    /// Reads a little-endian 32-bit float.
    #[inline]
    fn read_f32_le(&mut self) -> Result<f32> {
        read_binary_value!(self, f32, LittleEndian)
    }

    /// Reads a 64-bit float using a runtime byte order.
    #[inline]
    fn read_f64(&mut self, byte_order: ByteOrder) -> Result<f64> {
        match byte_order {
            ByteOrder::BigEndian => self.read_f64_be(),
            ByteOrder::LittleEndian => self.read_f64_le(),
        }
    }

    /// Reads a big-endian 64-bit float.
    #[inline]
    fn read_f64_be(&mut self) -> Result<f64> {
        read_binary_value!(self, f64, BigEndian)
    }

    /// Reads a little-endian 64-bit float.
    #[inline]
    fn read_f64_le(&mut self) -> Result<f64> {
        read_binary_value!(self, f64, LittleEndian)
    }
}

impl<R> BinaryReadExt for R where R: Read + ?Sized {}

#[inline]
fn read_binary<const N: usize, T, R, F>(reader: &mut R, decode: F) -> Result<T>
where
    R: Read + ?Sized,
    F: FnOnce(&[u8]) -> T,
{
    let mut bytes = [0u8; N];
    reader.read_exact(&mut bytes)?;
    Ok(decode(&bytes))
}
