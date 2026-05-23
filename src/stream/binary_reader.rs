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
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::ReadExt;
use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
    LittleEndian,
};
use crate::util::read_utf8_payload;

/// Reader wrapper for fixed-width binary values.
///
/// The byte order is selected by the `O` type parameter. Use
/// `BinaryReader<R, BigEndian>` for big-endian data and
/// `BinaryReader<R, LittleEndian>` for little-endian data.
pub struct BinaryReader<R, O = BigEndian> {
    inner: R,
    buffer: [u8; 16],
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
            buffer: [0; 16],
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

macro_rules! impl_value_read {
    ($order:ty, $method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self) -> Result<$ty> {
            type Codec = BinaryCodec<$ty, $order>;

            const LEN: usize = Codec::REQUIRED_MIN_BUFFER_LEN;
            // SAFETY: `LEN` is declared by the codec and fits the fixed internal buffer.
            unsafe {
                ReadExt::read_exact_unchecked(&mut self.inner, &mut self.buffer, 0, LEN)?;
                Ok(Codec::read_unchecked(&self.buffer, 0))
            }
        }
    };
}

macro_rules! impl_for_order {
    ($order:ty) => {
        impl<R> BinaryReader<R, $order>
        where
            R: Read,
        {
            impl_value_read!($order, read_u8, u8, "Reads an unsigned 8-bit integer.");
            impl_value_read!($order, read_i8, i8, "Reads a signed 8-bit integer.");
            impl_value_read!($order, read_u16, u16, "Reads an unsigned 16-bit integer.");
            impl_value_read!($order, read_u32, u32, "Reads an unsigned 32-bit integer.");
            impl_value_read!($order, read_u64, u64, "Reads an unsigned 64-bit integer.");
            impl_value_read!($order, read_u128, u128, "Reads an unsigned 128-bit integer.");
            impl_value_read!($order, read_i16, i16, "Reads a signed 16-bit integer.");
            impl_value_read!($order, read_i32, i32, "Reads a signed 32-bit integer.");
            impl_value_read!($order, read_i64, i64, "Reads a signed 64-bit integer.");
            impl_value_read!($order, read_i128, i128, "Reads a signed 128-bit integer.");
            impl_value_read!($order, read_f32, f32, "Reads a 32-bit float.");
            impl_value_read!($order, read_f64, f64, "Reads a 64-bit float.");

            /// Reads a UTF-8 string prefixed by a 16-bit byte length.
            ///
            /// # Parameters
            ///
            /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
            ///
            /// # Errors
            ///
            /// Returns [`std::io::ErrorKind::InvalidData`] when the encoded length exceeds
            /// `max_len` or when the payload is not valid UTF-8.
            #[inline]
            pub fn read_utf8_string_u16(&mut self, max_len: usize) -> Result<String> {
                let len = usize::from(self.read_u16()?);
                read_utf8_payload(&mut self.inner, len, max_len)
            }

            /// Reads a UTF-8 string prefixed by a 32-bit byte length.
            ///
            /// # Parameters
            ///
            /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
            ///
            /// # Errors
            ///
            /// Returns [`std::io::ErrorKind::InvalidData`] when the encoded length exceeds
            /// `max_len` or when the payload is not valid UTF-8.
            #[inline]
            pub fn read_utf8_string_u32(&mut self, max_len: usize) -> Result<String> {
                let len = self.read_u32()? as usize;
                read_utf8_payload(&mut self.inner, len, max_len)
            }
        }
    };
}

impl_for_order!(BigEndian);
impl_for_order!(LittleEndian);

impl<R, O> Read for BinaryReader<R, O>
where
    R: Read,
{
    /// Reads bytes from the wrapped reader.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Destination byte buffer.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns the I/O error reported by the wrapped reader.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}

impl<R, O> Seek for BinaryReader<R, O>
where
    R: Seek,
{
    /// Seeks the wrapped reader.
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
    /// Returns the seek error reported by the wrapped reader.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
