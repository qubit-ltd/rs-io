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
    Result,
    Seek,
    SeekFrom,
    Write,
};

use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
    LittleEndian,
};
use crate::util::{
    write_utf8_string_with_u16_len,
    write_utf8_string_with_u32_len,
};

macro_rules! write_binary_value {
    ($writer:expr, $value:expr, $ty:ty, $order:ty) => {
        write_binary::<{ BinaryCodec::<$ty, $order>::REQUIRED_MIN_BUFFER_LEN }, _, _, _>(
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
                write_utf8_string_with_u16_len(&mut self.inner, value, |writer, len| {
                    write_binary_value!(writer, len, u16, $order)
                })
            }

            /// Writes a UTF-8 string prefixed by a 32-bit byte length.
            #[inline]
            pub fn write_utf8_string_u32(&mut self, value: &str) -> Result<()> {
                write_utf8_string_with_u32_len(&mut self.inner, value, |writer, len| {
                    write_binary_value!(writer, len, u32, $order)
                })
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

impl<W, O> Write for BinaryWriter<W, O>
where
    W: Write,
{
    /// Writes bytes to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Source bytes to write.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped writer.
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.inner.write(buffer)
    }

    /// Flushes the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped writer.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W, O> Seek for BinaryWriter<W, O>
where
    W: Seek,
{
    /// Seeks the wrapped writer.
    ///
    /// # Parameters
    ///
    /// - `position`: Target seek position.
    ///
    /// # Returns
    ///
    /// Returns the new stream position.
    ///
    /// # Errors
    ///
    /// Returns the seek error reported by the wrapped writer.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}

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
