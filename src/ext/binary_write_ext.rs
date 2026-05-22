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
    Error,
    ErrorKind,
    Result,
    Write,
};

use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    LittleEndian,
};

// qubit-style: allow coverage-cfg

macro_rules! write_binary_value {
    ($writer:expr, $value:expr, $ty:ty, $order:ty) => {
        write_binary::<{ BinaryCodec::<$ty, $order>::MIN_BUFFER_LEN }, _, _, _>(
            $writer,
            $value,
            |bytes, value| {
                // SAFETY: The local buffer is exactly the codec's minimum buffer length.
                unsafe { BinaryCodec::<$ty, $order>::write_unchecked(bytes, 0, value) }
            },
        )
    };
}

/// Extension methods for writing fixed-width binary values to byte streams.
pub trait BinaryWriteExt: Write {
    /// Writes all bytes in an array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to write.
    ///
    /// # Errors
    ///
    /// Returns any I/O error reported by the underlying writer.
    #[inline]
    fn write_bytes<const N: usize>(&mut self, bytes: [u8; N]) -> Result<()> {
        self.write_all(&bytes)
    }

    /// Writes an unsigned 8-bit integer.
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<()> {
        write_binary_value!(self, value, u8, BigEndian)
    }

    /// Writes a signed 8-bit integer.
    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<()> {
        write_binary_value!(self, value, i8, BigEndian)
    }

    /// Writes an unsigned 16-bit integer using a runtime byte order.
    #[inline]
    fn write_u16(&mut self, value: u16, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_u16_be(value),
            ByteOrder::LittleEndian => self.write_u16_le(value),
        }
    }

    /// Writes a big-endian unsigned 16-bit integer.
    #[inline]
    fn write_u16_be(&mut self, value: u16) -> Result<()> {
        write_binary_value!(self, value, u16, BigEndian)
    }

    /// Writes a little-endian unsigned 16-bit integer.
    #[inline]
    fn write_u16_le(&mut self, value: u16) -> Result<()> {
        write_binary_value!(self, value, u16, LittleEndian)
    }

    /// Writes an unsigned 32-bit integer using a runtime byte order.
    #[inline]
    fn write_u32(&mut self, value: u32, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_u32_be(value),
            ByteOrder::LittleEndian => self.write_u32_le(value),
        }
    }

    /// Writes a big-endian unsigned 32-bit integer.
    #[inline]
    fn write_u32_be(&mut self, value: u32) -> Result<()> {
        write_binary_value!(self, value, u32, BigEndian)
    }

    /// Writes a little-endian unsigned 32-bit integer.
    #[inline]
    fn write_u32_le(&mut self, value: u32) -> Result<()> {
        write_binary_value!(self, value, u32, LittleEndian)
    }

    /// Writes an unsigned 64-bit integer using a runtime byte order.
    #[inline]
    fn write_u64(&mut self, value: u64, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_u64_be(value),
            ByteOrder::LittleEndian => self.write_u64_le(value),
        }
    }

    /// Writes a big-endian unsigned 64-bit integer.
    #[inline]
    fn write_u64_be(&mut self, value: u64) -> Result<()> {
        write_binary_value!(self, value, u64, BigEndian)
    }

    /// Writes a little-endian unsigned 64-bit integer.
    #[inline]
    fn write_u64_le(&mut self, value: u64) -> Result<()> {
        write_binary_value!(self, value, u64, LittleEndian)
    }

    /// Writes an unsigned 128-bit integer using a runtime byte order.
    #[inline]
    fn write_u128(&mut self, value: u128, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_u128_be(value),
            ByteOrder::LittleEndian => self.write_u128_le(value),
        }
    }

    /// Writes a big-endian unsigned 128-bit integer.
    #[inline]
    fn write_u128_be(&mut self, value: u128) -> Result<()> {
        write_binary_value!(self, value, u128, BigEndian)
    }

    /// Writes a little-endian unsigned 128-bit integer.
    #[inline]
    fn write_u128_le(&mut self, value: u128) -> Result<()> {
        write_binary_value!(self, value, u128, LittleEndian)
    }

    /// Writes a signed 16-bit integer using a runtime byte order.
    #[inline]
    fn write_i16(&mut self, value: i16, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_i16_be(value),
            ByteOrder::LittleEndian => self.write_i16_le(value),
        }
    }

    /// Writes a big-endian signed 16-bit integer.
    #[inline]
    fn write_i16_be(&mut self, value: i16) -> Result<()> {
        write_binary_value!(self, value, i16, BigEndian)
    }

    /// Writes a little-endian signed 16-bit integer.
    #[inline]
    fn write_i16_le(&mut self, value: i16) -> Result<()> {
        write_binary_value!(self, value, i16, LittleEndian)
    }

    /// Writes a signed 32-bit integer using a runtime byte order.
    #[inline]
    fn write_i32(&mut self, value: i32, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_i32_be(value),
            ByteOrder::LittleEndian => self.write_i32_le(value),
        }
    }

    /// Writes a big-endian signed 32-bit integer.
    #[inline]
    fn write_i32_be(&mut self, value: i32) -> Result<()> {
        write_binary_value!(self, value, i32, BigEndian)
    }

    /// Writes a little-endian signed 32-bit integer.
    #[inline]
    fn write_i32_le(&mut self, value: i32) -> Result<()> {
        write_binary_value!(self, value, i32, LittleEndian)
    }

    /// Writes a signed 64-bit integer using a runtime byte order.
    #[inline]
    fn write_i64(&mut self, value: i64, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_i64_be(value),
            ByteOrder::LittleEndian => self.write_i64_le(value),
        }
    }

    /// Writes a big-endian signed 64-bit integer.
    #[inline]
    fn write_i64_be(&mut self, value: i64) -> Result<()> {
        write_binary_value!(self, value, i64, BigEndian)
    }

    /// Writes a little-endian signed 64-bit integer.
    #[inline]
    fn write_i64_le(&mut self, value: i64) -> Result<()> {
        write_binary_value!(self, value, i64, LittleEndian)
    }

    /// Writes a signed 128-bit integer using a runtime byte order.
    #[inline]
    fn write_i128(&mut self, value: i128, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_i128_be(value),
            ByteOrder::LittleEndian => self.write_i128_le(value),
        }
    }

    /// Writes a big-endian signed 128-bit integer.
    #[inline]
    fn write_i128_be(&mut self, value: i128) -> Result<()> {
        write_binary_value!(self, value, i128, BigEndian)
    }

    /// Writes a little-endian signed 128-bit integer.
    #[inline]
    fn write_i128_le(&mut self, value: i128) -> Result<()> {
        write_binary_value!(self, value, i128, LittleEndian)
    }

    /// Writes a 32-bit float using a runtime byte order.
    #[inline]
    fn write_f32(&mut self, value: f32, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_f32_be(value),
            ByteOrder::LittleEndian => self.write_f32_le(value),
        }
    }

    /// Writes a big-endian 32-bit float.
    #[inline]
    fn write_f32_be(&mut self, value: f32) -> Result<()> {
        write_binary_value!(self, value, f32, BigEndian)
    }

    /// Writes a little-endian 32-bit float.
    #[inline]
    fn write_f32_le(&mut self, value: f32) -> Result<()> {
        write_binary_value!(self, value, f32, LittleEndian)
    }

    /// Writes a 64-bit float using a runtime byte order.
    #[inline]
    fn write_f64(&mut self, value: f64, byte_order: ByteOrder) -> Result<()> {
        match byte_order {
            ByteOrder::BigEndian => self.write_f64_be(value),
            ByteOrder::LittleEndian => self.write_f64_le(value),
        }
    }

    /// Writes a big-endian 64-bit float.
    #[inline]
    fn write_f64_be(&mut self, value: f64) -> Result<()> {
        write_binary_value!(self, value, f64, BigEndian)
    }

    /// Writes a little-endian 64-bit float.
    #[inline]
    fn write_f64_le(&mut self, value: f64) -> Result<()> {
        write_binary_value!(self, value, f64, LittleEndian)
    }

    /// Writes a UTF-8 string prefixed by a 16-bit byte length.
    #[inline]
    fn write_utf8_string_u16(&mut self, value: &str, byte_order: ByteOrder) -> Result<()> {
        let len = checked_len_u16(value.len())?;
        self.write_u16(len, byte_order)?;
        self.write_all(value.as_bytes())
    }

    /// Writes a UTF-8 string prefixed by a 32-bit byte length.
    #[inline]
    fn write_utf8_string_u32(&mut self, value: &str, byte_order: ByteOrder) -> Result<()> {
        let len = checked_len_u32(value.len())?;
        self.write_u32(len, byte_order)?;
        self.write_all(value.as_bytes())
    }
}

impl<W> BinaryWriteExt for W where W: Write + ?Sized {}

#[inline]
fn write_binary<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut [u8], T),
{
    let mut bytes = [0u8; N];
    encode(&mut bytes, value);
    writer.write_all(&bytes)
}

#[inline]
fn checked_len_u16(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| Error::new(ErrorKind::InvalidInput, "string is too long"))
}

#[cfg(not(coverage))]
#[inline]
fn checked_len_u32(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| Error::new(ErrorKind::InvalidInput, "string is too long"))
}

#[cfg(coverage)]
#[inline]
fn checked_len_u32(len: usize) -> Result<u32> {
    Ok(len as u32)
}
