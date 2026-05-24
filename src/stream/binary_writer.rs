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

use crate::WriteExt;
use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
    LittleEndian,
};
use crate::util::{
    checked_u16_len,
    checked_u32_len,
};

/// Writer wrapper for fixed-width binary values.
///
/// The byte order is selected by the `O` type parameter. Use
/// `BinaryWriter<W, BigEndian>` for big-endian data and
/// `BinaryWriter<W, LittleEndian>` for little-endian data.
pub struct BinaryWriter<W, O = BigEndian> {
    inner: W,
    buffer: [u8; 16],
    marker: PhantomData<fn() -> O>,
}

impl<W, O> BinaryWriter<W, O>
where
    W: Write,
    O: ByteOrderSpec,
{
    /// Creates a binary writer.
    ///
    /// # Parameters
    ///
    /// - `inner`: Underlying byte writer.
    ///
    /// # Returns
    ///
    /// Returns a writer using the byte order selected by `O`.
    #[must_use]
    #[inline]
    pub const fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: [0; 16],
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

macro_rules! impl_value_write {
    ($order:ty, $method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self, value: $ty) -> Result<()> {
            type Codec = BinaryCodec<$ty, $order>;

            const LEN: usize = Codec::REQUIRED_MIN_BUFFER_LEN;
            // SAFETY: `LEN` is declared by the codec and fits the fixed internal buffer.
            unsafe {
                Codec::write_unchecked(&mut self.buffer, 0, value);
                self.inner.write_all_unchecked(&self.buffer, 0, LEN)
            }
        }
    };
}

macro_rules! impl_for_order {
    ($order:ty) => {
        impl<W> BinaryWriter<W, $order>
        where
            W: Write,
        {
            impl_value_write!($order, write_u8, u8, "Writes an unsigned 8-bit integer.");
            impl_value_write!($order, write_i8, i8, "Writes a signed 8-bit integer.");
            impl_value_write!($order, write_u16, u16, "Writes an unsigned 16-bit integer.");
            impl_value_write!($order, write_u32, u32, "Writes an unsigned 32-bit integer.");
            impl_value_write!($order, write_u64, u64, "Writes an unsigned 64-bit integer.");
            impl_value_write!($order, write_u128, u128, "Writes an unsigned 128-bit integer.");
            impl_value_write!($order, write_i16, i16, "Writes a signed 16-bit integer.");
            impl_value_write!($order, write_i32, i32, "Writes a signed 32-bit integer.");
            impl_value_write!($order, write_i64, i64, "Writes a signed 64-bit integer.");
            impl_value_write!($order, write_i128, i128, "Writes a signed 128-bit integer.");
            impl_value_write!($order, write_f32, f32, "Writes a 32-bit float.");
            impl_value_write!($order, write_f64, f64, "Writes a 64-bit float.");

            /// Writes a UTF-8 string prefixed by a 16-bit byte length.
            #[inline]
            pub fn write_utf8_string_u16(&mut self, value: &str) -> Result<()> {
                self.write_u16(checked_u16_len(value.len())?)?;
                let bytes = value.as_bytes();
                // SAFETY: The range covers the full byte slice produced by `str::as_bytes`.
                unsafe { self.inner.write_all_unchecked(bytes, 0, bytes.len()) }
            }

            /// Writes a UTF-8 string prefixed by a 32-bit byte length.
            #[inline]
            pub fn write_utf8_string_u32(&mut self, value: &str) -> Result<()> {
                self.write_u32(checked_u32_len(value.len())?)?;
                let bytes = value.as_bytes();
                // SAFETY: The range covers the full byte slice produced by `str::as_bytes`.
                unsafe { self.inner.write_all_unchecked(bytes, 0, bytes.len()) }
            }
        }
    };
}

impl_for_order!(BigEndian);
impl_for_order!(LittleEndian);

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
