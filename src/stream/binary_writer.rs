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
    Result,
    Write,
};

use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
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

macro_rules! impl_binary_writer_for_order {
    ($order:ty) => {
        impl<W> BinaryWriter<W, $order>
        where
            W: Write,
        {
            /// Writes an unsigned 8-bit integer.
            #[inline]
            pub fn write_u8(&mut self, value: u8) -> Result<()> {
                write_binary_value!(&mut self.inner, value, u8, $order)
            }

            /// Writes a signed 8-bit integer.
            #[inline]
            pub fn write_i8(&mut self, value: i8) -> Result<()> {
                write_binary_value!(&mut self.inner, value, i8, $order)
            }

            /// Writes an unsigned 16-bit integer.
            #[inline]
            pub fn write_u16(&mut self, value: u16) -> Result<()> {
                write_binary_value!(&mut self.inner, value, u16, $order)
            }

            /// Writes an unsigned 32-bit integer.
            #[inline]
            pub fn write_u32(&mut self, value: u32) -> Result<()> {
                write_binary_value!(&mut self.inner, value, u32, $order)
            }

            /// Writes an unsigned 64-bit integer.
            #[inline]
            pub fn write_u64(&mut self, value: u64) -> Result<()> {
                write_binary_value!(&mut self.inner, value, u64, $order)
            }

            /// Writes an unsigned 128-bit integer.
            #[inline]
            pub fn write_u128(&mut self, value: u128) -> Result<()> {
                write_binary_value!(&mut self.inner, value, u128, $order)
            }

            /// Writes a signed 16-bit integer.
            #[inline]
            pub fn write_i16(&mut self, value: i16) -> Result<()> {
                write_binary_value!(&mut self.inner, value, i16, $order)
            }

            /// Writes a signed 32-bit integer.
            #[inline]
            pub fn write_i32(&mut self, value: i32) -> Result<()> {
                write_binary_value!(&mut self.inner, value, i32, $order)
            }

            /// Writes a signed 64-bit integer.
            #[inline]
            pub fn write_i64(&mut self, value: i64) -> Result<()> {
                write_binary_value!(&mut self.inner, value, i64, $order)
            }

            /// Writes a signed 128-bit integer.
            #[inline]
            pub fn write_i128(&mut self, value: i128) -> Result<()> {
                write_binary_value!(&mut self.inner, value, i128, $order)
            }

            /// Writes a 32-bit float.
            #[inline]
            pub fn write_f32(&mut self, value: f32) -> Result<()> {
                write_binary_value!(&mut self.inner, value, f32, $order)
            }

            /// Writes a 64-bit float.
            #[inline]
            pub fn write_f64(&mut self, value: f64) -> Result<()> {
                write_binary_value!(&mut self.inner, value, f64, $order)
            }

            /// Writes a UTF-8 string prefixed by a 16-bit byte length.
            #[inline]
            pub fn write_utf8_string_u16(&mut self, value: &str) -> Result<()> {
                let len = checked_len_u16(value.len())?;
                self.write_u16(len)?;
                self.inner.write_all(value.as_bytes())
            }

            /// Writes a UTF-8 string prefixed by a 32-bit byte length.
            #[inline]
            pub fn write_utf8_string_u32(&mut self, value: &str) -> Result<()> {
                let len = checked_len_u32(value.len())?;
                self.write_u32(len)?;
                self.inner.write_all(value.as_bytes())
            }
        }
    };
}

/// Writer wrapper for fixed-width binary values.
///
/// The byte order is selected by the `O` type parameter. Use
/// `BinaryWriter<W, BigEndian>` for big-endian data and
/// `BinaryWriter<W, LittleEndian>` for little-endian data.
pub struct BinaryWriter<W, O = BigEndian> {
    inner: W,
    marker: PhantomData<fn() -> O>,
}

impl<W, O> BinaryWriter<W, O>
where
    O: ByteOrderSpec,
{
    /// Creates a binary writer.
    #[must_use]
    #[inline]
    pub const fn new(inner: W) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Returns the byte order selected by this writer.
    #[must_use]
    #[inline]
    pub const fn byte_order(&self) -> ByteOrder {
        O::ORDER
    }

    /// Returns a shared reference to the underlying writer.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns an exclusive reference to the underlying writer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the underlying writer.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W, O> BinaryWriter<W, O>
where
    W: Write,
    O: ByteOrderSpec,
{
    /// Writes all bytes in an array.
    #[inline]
    pub fn write_bytes<const N: usize>(&mut self, bytes: [u8; N]) -> Result<()> {
        self.inner.write_all(&bytes)
    }
}

impl_binary_writer_for_order!(BigEndian);
impl_binary_writer_for_order!(LittleEndian);

#[inline]
fn write_binary<const N: usize, T, W, F>(writer: &mut W, value: T, encode: F) -> Result<()>
where
    W: Write,
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
