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

use crate::ByteOrder;

/// Extension methods for writing binary scalar values to [`Write`] streams.
///
/// Multi-byte values can be written either with explicit byte-order suffixes
/// or with a runtime [`ByteOrder`] argument. All methods delegate to
/// [`Write::write_all`], so they either write the complete encoded value or
/// return the first I/O error.
pub trait BinaryWriteExt: Write {
    /// Writes one unsigned byte.
    ///
    /// # Parameters
    /// - `value`: Byte to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u8(&mut self, value: u8) -> Result<()>;

    /// Writes one signed byte.
    ///
    /// # Parameters
    /// - `value`: Byte to write.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i8(&mut self, value: i8) -> Result<()>;

    /// Writes a `u16` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_u16(&mut self, value: u16, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_u16_be(value),
            ByteOrder::LittleEndian => self.write_u16_le(value),
        }
    }

    /// Writes a big-endian `u16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u16_be(&mut self, value: u16) -> Result<()>;

    /// Writes a little-endian `u16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u16_le(&mut self, value: u16) -> Result<()>;

    /// Writes an `i16` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_i16(&mut self, value: i16, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_i16_be(value),
            ByteOrder::LittleEndian => self.write_i16_le(value),
        }
    }

    /// Writes a big-endian `i16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i16_be(&mut self, value: i16) -> Result<()>;

    /// Writes a little-endian `i16`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i16_le(&mut self, value: i16) -> Result<()>;

    /// Writes a `u32` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_u32(&mut self, value: u32, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_u32_be(value),
            ByteOrder::LittleEndian => self.write_u32_le(value),
        }
    }

    /// Writes a big-endian `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u32_be(&mut self, value: u32) -> Result<()>;

    /// Writes a little-endian `u32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u32_le(&mut self, value: u32) -> Result<()>;

    /// Writes an `i32` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_i32(&mut self, value: i32, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_i32_be(value),
            ByteOrder::LittleEndian => self.write_i32_le(value),
        }
    }

    /// Writes a big-endian `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i32_be(&mut self, value: i32) -> Result<()>;

    /// Writes a little-endian `i32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i32_le(&mut self, value: i32) -> Result<()>;

    /// Writes a `u64` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_u64(&mut self, value: u64, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_u64_be(value),
            ByteOrder::LittleEndian => self.write_u64_le(value),
        }
    }

    /// Writes a big-endian `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u64_be(&mut self, value: u64) -> Result<()>;

    /// Writes a little-endian `u64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u64_le(&mut self, value: u64) -> Result<()>;

    /// Writes an `i64` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_i64(&mut self, value: i64, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_i64_be(value),
            ByteOrder::LittleEndian => self.write_i64_le(value),
        }
    }

    /// Writes a big-endian `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i64_be(&mut self, value: i64) -> Result<()>;

    /// Writes a little-endian `i64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i64_le(&mut self, value: i64) -> Result<()>;

    /// Writes a `u128` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_u128(&mut self, value: u128, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_u128_be(value),
            ByteOrder::LittleEndian => self.write_u128_le(value),
        }
    }

    /// Writes a big-endian `u128`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u128_be(&mut self, value: u128) -> Result<()>;

    /// Writes a little-endian `u128`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_u128_le(&mut self, value: u128) -> Result<()>;

    /// Writes an `i128` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_i128(&mut self, value: i128, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_i128_be(value),
            ByteOrder::LittleEndian => self.write_i128_le(value),
        }
    }

    /// Writes a big-endian `i128`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i128_be(&mut self, value: i128) -> Result<()>;

    /// Writes a little-endian `i128`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_i128_le(&mut self, value: i128) -> Result<()>;

    /// Writes an IEEE-754 `f32` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_f32(&mut self, value: f32, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_f32_be(value),
            ByteOrder::LittleEndian => self.write_f32_le(value),
        }
    }

    /// Writes a big-endian IEEE-754 `f32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f32_be(&mut self, value: f32) -> Result<()>;

    /// Writes a little-endian IEEE-754 `f32`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f32_le(&mut self, value: f32) -> Result<()>;

    /// Writes an IEEE-754 `f64` using `order`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    /// - `order`: Byte order used to encode the value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    #[inline]
    fn write_f64(&mut self, value: f64, order: ByteOrder) -> Result<()> {
        match order {
            ByteOrder::BigEndian => self.write_f64_be(value),
            ByteOrder::LittleEndian => self.write_f64_le(value),
        }
    }

    /// Writes a big-endian IEEE-754 `f64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f64_be(&mut self, value: f64) -> Result<()>;

    /// Writes a little-endian IEEE-754 `f64`.
    ///
    /// # Parameters
    /// - `value`: Value to encode.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying writer.
    fn write_f64_le(&mut self, value: f64) -> Result<()>;
}

impl<T> BinaryWriteExt for T
where
    T: Write + ?Sized,
{
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write_all(&[value])
    }

    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_all(&[value as u8])
    }

    #[inline]
    fn write_u16_be(&mut self, value: u16) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u16_le(&mut self, value: u16) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i16_be(&mut self, value: i16) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i16_le(&mut self, value: i16) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_u32_be(&mut self, value: u32) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u32_le(&mut self, value: u32) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i32_be(&mut self, value: i32) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i32_le(&mut self, value: i32) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_u64_be(&mut self, value: u64) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u64_le(&mut self, value: u64) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i64_be(&mut self, value: i64) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i64_le(&mut self, value: i64) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_u128_be(&mut self, value: u128) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_u128_le(&mut self, value: u128) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_i128_be(&mut self, value: i128) -> Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    #[inline]
    fn write_i128_le(&mut self, value: i128) -> Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    #[inline]
    fn write_f32_be(&mut self, value: f32) -> Result<()> {
        self.write_u32_be(value.to_bits())
    }

    #[inline]
    fn write_f32_le(&mut self, value: f32) -> Result<()> {
        self.write_u32_le(value.to_bits())
    }

    #[inline]
    fn write_f64_be(&mut self, value: f64) -> Result<()> {
        self.write_u64_be(value.to_bits())
    }

    #[inline]
    fn write_f64_le(&mut self, value: f64) -> Result<()> {
        self.write_u64_le(value.to_bits())
    }
}
