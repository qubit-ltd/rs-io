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

use crate::ByteOrder;

/// Extension methods for reading binary scalar values from [`Read`] streams.
///
/// Multi-byte values can be read either with explicit byte-order suffixes or
/// with a runtime [`ByteOrder`] argument. All methods read exactly the required
/// number of bytes and therefore return [`std::io::ErrorKind::UnexpectedEof`]
/// when the stream ends early.
pub trait BinaryReadExt: Read {
    /// Reads one unsigned byte.
    ///
    /// # Returns
    /// The byte value.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying reader, including
    /// `UnexpectedEof` when no byte is available.
    fn read_u8(&mut self) -> Result<u8>;

    /// Reads one signed byte.
    ///
    /// # Returns
    /// The byte interpreted as `i8`.
    ///
    /// # Errors
    /// Returns an I/O error from the underlying reader, including
    /// `UnexpectedEof` when no byte is available.
    fn read_i8(&mut self) -> Result<i8>;

    /// Reads a `u16` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    #[inline]
    fn read_u16(&mut self, order: ByteOrder) -> Result<u16> {
        match order {
            ByteOrder::BigEndian => self.read_u16_be(),
            ByteOrder::LittleEndian => self.read_u16_le(),
        }
    }

    /// Reads a big-endian `u16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    fn read_u16_be(&mut self) -> Result<u16>;

    /// Reads a little-endian `u16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    fn read_u16_le(&mut self) -> Result<u16>;

    /// Reads an `i16` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    #[inline]
    fn read_i16(&mut self, order: ByteOrder) -> Result<i16> {
        match order {
            ByteOrder::BigEndian => self.read_i16_be(),
            ByteOrder::LittleEndian => self.read_i16_le(),
        }
    }

    /// Reads a big-endian `i16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    fn read_i16_be(&mut self) -> Result<i16>;

    /// Reads a little-endian `i16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when two bytes cannot be read.
    fn read_i16_le(&mut self) -> Result<i16>;

    /// Reads a `u32` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    #[inline]
    fn read_u32(&mut self, order: ByteOrder) -> Result<u32> {
        match order {
            ByteOrder::BigEndian => self.read_u32_be(),
            ByteOrder::LittleEndian => self.read_u32_le(),
        }
    }

    /// Reads a big-endian `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_u32_be(&mut self) -> Result<u32>;

    /// Reads a little-endian `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_u32_le(&mut self) -> Result<u32>;

    /// Reads an `i32` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    #[inline]
    fn read_i32(&mut self, order: ByteOrder) -> Result<i32> {
        match order {
            ByteOrder::BigEndian => self.read_i32_be(),
            ByteOrder::LittleEndian => self.read_i32_le(),
        }
    }

    /// Reads a big-endian `i32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_i32_be(&mut self) -> Result<i32>;

    /// Reads a little-endian `i32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_i32_le(&mut self) -> Result<i32>;

    /// Reads a `u64` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    #[inline]
    fn read_u64(&mut self, order: ByteOrder) -> Result<u64> {
        match order {
            ByteOrder::BigEndian => self.read_u64_be(),
            ByteOrder::LittleEndian => self.read_u64_le(),
        }
    }

    /// Reads a big-endian `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_u64_be(&mut self) -> Result<u64>;

    /// Reads a little-endian `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_u64_le(&mut self) -> Result<u64>;

    /// Reads an `i64` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    #[inline]
    fn read_i64(&mut self, order: ByteOrder) -> Result<i64> {
        match order {
            ByteOrder::BigEndian => self.read_i64_be(),
            ByteOrder::LittleEndian => self.read_i64_le(),
        }
    }

    /// Reads a big-endian `i64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_i64_be(&mut self) -> Result<i64>;

    /// Reads a little-endian `i64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_i64_le(&mut self) -> Result<i64>;

    /// Reads a `u128` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    #[inline]
    fn read_u128(&mut self, order: ByteOrder) -> Result<u128> {
        match order {
            ByteOrder::BigEndian => self.read_u128_be(),
            ByteOrder::LittleEndian => self.read_u128_le(),
        }
    }

    /// Reads a big-endian `u128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    fn read_u128_be(&mut self) -> Result<u128>;

    /// Reads a little-endian `u128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    fn read_u128_le(&mut self) -> Result<u128>;

    /// Reads an `i128` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    #[inline]
    fn read_i128(&mut self, order: ByteOrder) -> Result<i128> {
        match order {
            ByteOrder::BigEndian => self.read_i128_be(),
            ByteOrder::LittleEndian => self.read_i128_le(),
        }
    }

    /// Reads a big-endian `i128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    fn read_i128_be(&mut self) -> Result<i128>;

    /// Reads a little-endian `i128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when sixteen bytes cannot be read.
    fn read_i128_le(&mut self) -> Result<i128>;

    /// Reads an IEEE-754 `f32` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    #[inline]
    fn read_f32(&mut self, order: ByteOrder) -> Result<f32> {
        match order {
            ByteOrder::BigEndian => self.read_f32_be(),
            ByteOrder::LittleEndian => self.read_f32_le(),
        }
    }

    /// Reads a big-endian IEEE-754 `f32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_f32_be(&mut self) -> Result<f32>;

    /// Reads a little-endian IEEE-754 `f32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when four bytes cannot be read.
    fn read_f32_le(&mut self) -> Result<f32>;

    /// Reads an IEEE-754 `f64` using `order`.
    ///
    /// # Parameters
    /// - `order`: Byte order used to decode the value.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    #[inline]
    fn read_f64(&mut self, order: ByteOrder) -> Result<f64> {
        match order {
            ByteOrder::BigEndian => self.read_f64_be(),
            ByteOrder::LittleEndian => self.read_f64_le(),
        }
    }

    /// Reads a big-endian IEEE-754 `f64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_f64_be(&mut self) -> Result<f64>;

    /// Reads a little-endian IEEE-754 `f64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns an I/O error when eight bytes cannot be read.
    fn read_f64_le(&mut self) -> Result<f64>;
}

impl<T> BinaryReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        read_bytes::<_, 1>(self).map(|buffer| buffer[0])
    }

    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        read_bytes::<_, 1>(self).map(|buffer| buffer[0] as i8)
    }

    #[inline]
    fn read_u16_be(&mut self) -> Result<u16> {
        read_bytes::<_, 2>(self).map(u16::from_be_bytes)
    }

    #[inline]
    fn read_u16_le(&mut self) -> Result<u16> {
        read_bytes::<_, 2>(self).map(u16::from_le_bytes)
    }

    #[inline]
    fn read_i16_be(&mut self) -> Result<i16> {
        read_bytes::<_, 2>(self).map(i16::from_be_bytes)
    }

    #[inline]
    fn read_i16_le(&mut self) -> Result<i16> {
        read_bytes::<_, 2>(self).map(i16::from_le_bytes)
    }

    #[inline]
    fn read_u32_be(&mut self) -> Result<u32> {
        read_bytes::<_, 4>(self).map(u32::from_be_bytes)
    }

    #[inline]
    fn read_u32_le(&mut self) -> Result<u32> {
        read_bytes::<_, 4>(self).map(u32::from_le_bytes)
    }

    #[inline]
    fn read_i32_be(&mut self) -> Result<i32> {
        read_bytes::<_, 4>(self).map(i32::from_be_bytes)
    }

    #[inline]
    fn read_i32_le(&mut self) -> Result<i32> {
        read_bytes::<_, 4>(self).map(i32::from_le_bytes)
    }

    #[inline]
    fn read_u64_be(&mut self) -> Result<u64> {
        read_bytes::<_, 8>(self).map(u64::from_be_bytes)
    }

    #[inline]
    fn read_u64_le(&mut self) -> Result<u64> {
        read_bytes::<_, 8>(self).map(u64::from_le_bytes)
    }

    #[inline]
    fn read_i64_be(&mut self) -> Result<i64> {
        read_bytes::<_, 8>(self).map(i64::from_be_bytes)
    }

    #[inline]
    fn read_i64_le(&mut self) -> Result<i64> {
        read_bytes::<_, 8>(self).map(i64::from_le_bytes)
    }

    #[inline]
    fn read_u128_be(&mut self) -> Result<u128> {
        read_bytes::<_, 16>(self).map(u128::from_be_bytes)
    }

    #[inline]
    fn read_u128_le(&mut self) -> Result<u128> {
        read_bytes::<_, 16>(self).map(u128::from_le_bytes)
    }

    #[inline]
    fn read_i128_be(&mut self) -> Result<i128> {
        read_bytes::<_, 16>(self).map(i128::from_be_bytes)
    }

    #[inline]
    fn read_i128_le(&mut self) -> Result<i128> {
        read_bytes::<_, 16>(self).map(i128::from_le_bytes)
    }

    #[inline]
    fn read_f32_be(&mut self) -> Result<f32> {
        read_bytes::<_, 4>(self).map(|buffer| f32::from_bits(u32::from_be_bytes(buffer)))
    }

    #[inline]
    fn read_f32_le(&mut self) -> Result<f32> {
        read_bytes::<_, 4>(self).map(|buffer| f32::from_bits(u32::from_le_bytes(buffer)))
    }

    #[inline]
    fn read_f64_be(&mut self) -> Result<f64> {
        read_bytes::<_, 8>(self).map(|buffer| f64::from_bits(u64::from_be_bytes(buffer)))
    }

    #[inline]
    fn read_f64_le(&mut self) -> Result<f64> {
        read_bytes::<_, 8>(self).map(|buffer| f64::from_bits(u64::from_le_bytes(buffer)))
    }
}

/// Reads exactly `N` bytes from `reader`.
///
/// # Parameters
/// - `reader`: Source reader. It may be a sized reader or a reader trait
///   object.
///
/// # Returns
/// An array containing the bytes read from the stream.
///
/// # Errors
/// Returns an I/O error from [`Read::read_exact`], including
/// [`std::io::ErrorKind::UnexpectedEof`] when fewer than `N` bytes are
/// available.
fn read_bytes<R, const N: usize>(reader: &mut R) -> Result<[u8; N]>
where
    R: Read + ?Sized,
{
    let mut buffer = [0; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}
