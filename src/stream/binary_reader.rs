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
    Error,
    ErrorKind,
    Read,
    Result,
};

use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
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

macro_rules! impl_binary_reader_for_order {
    ($order:ty) => {
        impl<R> BinaryReader<R, $order>
        where
            R: Read,
        {
            /// Reads an unsigned 8-bit integer.
            #[inline]
            pub fn read_u8(&mut self) -> Result<u8> {
                read_binary_value!(&mut self.inner, u8, $order)
            }

            /// Reads a signed 8-bit integer.
            #[inline]
            pub fn read_i8(&mut self) -> Result<i8> {
                read_binary_value!(&mut self.inner, i8, $order)
            }

            /// Reads an unsigned 16-bit integer.
            #[inline]
            pub fn read_u16(&mut self) -> Result<u16> {
                read_binary_value!(&mut self.inner, u16, $order)
            }

            /// Reads an unsigned 32-bit integer.
            #[inline]
            pub fn read_u32(&mut self) -> Result<u32> {
                read_binary_value!(&mut self.inner, u32, $order)
            }

            /// Reads an unsigned 64-bit integer.
            #[inline]
            pub fn read_u64(&mut self) -> Result<u64> {
                read_binary_value!(&mut self.inner, u64, $order)
            }

            /// Reads an unsigned 128-bit integer.
            #[inline]
            pub fn read_u128(&mut self) -> Result<u128> {
                read_binary_value!(&mut self.inner, u128, $order)
            }

            /// Reads a signed 16-bit integer.
            #[inline]
            pub fn read_i16(&mut self) -> Result<i16> {
                read_binary_value!(&mut self.inner, i16, $order)
            }

            /// Reads a signed 32-bit integer.
            #[inline]
            pub fn read_i32(&mut self) -> Result<i32> {
                read_binary_value!(&mut self.inner, i32, $order)
            }

            /// Reads a signed 64-bit integer.
            #[inline]
            pub fn read_i64(&mut self) -> Result<i64> {
                read_binary_value!(&mut self.inner, i64, $order)
            }

            /// Reads a signed 128-bit integer.
            #[inline]
            pub fn read_i128(&mut self) -> Result<i128> {
                read_binary_value!(&mut self.inner, i128, $order)
            }

            /// Reads a 32-bit float.
            #[inline]
            pub fn read_f32(&mut self) -> Result<f32> {
                read_binary_value!(&mut self.inner, f32, $order)
            }

            /// Reads a 64-bit float.
            #[inline]
            pub fn read_f64(&mut self) -> Result<f64> {
                read_binary_value!(&mut self.inner, f64, $order)
            }

            /// Reads a UTF-8 string prefixed by a 16-bit byte length.
            #[inline]
            pub fn read_utf8_string_u16(&mut self) -> Result<String> {
                let len = usize::from(self.read_u16()?);
                read_utf8_string(&mut self.inner, len)
            }

            /// Reads a UTF-8 string prefixed by a 32-bit byte length.
            #[inline]
            pub fn read_utf8_string_u32(&mut self) -> Result<String> {
                let len = self.read_u32()? as usize;
                read_utf8_string(&mut self.inner, len)
            }
        }
    };
}

/// Reader wrapper for fixed-width binary values.
///
/// The byte order is selected by the `O` type parameter. Use
/// `BinaryReader<R, BigEndian>` for big-endian data and
/// `BinaryReader<R, LittleEndian>` for little-endian data.
pub struct BinaryReader<R, O = BigEndian> {
    inner: R,
    marker: PhantomData<fn() -> O>,
}

impl<R, O> BinaryReader<R, O>
where
    O: ByteOrderSpec,
{
    /// Creates a binary reader.
    ///
    /// # Parameters
    ///
    /// - `inner`: Underlying byte reader.
    ///
    /// # Returns
    ///
    /// Returns a reader using the byte order selected by `O`.
    #[must_use]
    #[inline]
    pub const fn new(inner: R) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Returns the byte order selected by this reader.
    #[must_use]
    #[inline]
    pub const fn byte_order(&self) -> ByteOrder {
        O::ORDER
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

impl<R, O> BinaryReader<R, O>
where
    R: Read,
    O: ByteOrderSpec,
{
    /// Reads exactly `N` bytes.
    #[inline]
    pub fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut bytes = [0u8; N];
        self.inner.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

impl_binary_reader_for_order!(BigEndian);
impl_binary_reader_for_order!(LittleEndian);

#[inline]
fn read_binary<const N: usize, T, R, F>(reader: &mut R, decode: F) -> Result<T>
where
    R: Read,
    F: FnOnce(&[u8]) -> T,
{
    let mut bytes = [0u8; N];
    reader.read_exact(&mut bytes)?;
    Ok(decode(&bytes))
}

#[inline]
fn read_utf8_string<R>(reader: &mut R, len: usize) -> Result<String>
where
    R: Read,
{
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|error| Error::new(ErrorKind::InvalidData, error))
}
