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

use crate::codec::{
    BigEndian,
    BinaryCodec,
    ByteOrder,
    ByteOrderSpec,
    LittleEndian,
};
use crate::stream::BufferedInput;

/// Buffered reader for fixed-width binary values.
///
/// Scalar reads decode directly from the internal input buffer whenever enough
/// bytes are available, avoiding the per-value temporary buffer used by the
/// extension trait helpers.
///
/// # Buffered state
///
/// This reader may prefetch bytes from the wrapped reader. As a result,
/// [`Self::get_ref`] can observe an underlying stream position ahead of the
/// logical position exposed by this wrapper, and [`Self::into_inner`] discards
/// any prefetched bytes that have not been consumed.
pub struct BufferedBinaryReader<R, O = BigEndian> {
    input: BufferedInput<R>,
    marker: PhantomData<fn() -> O>,
}

impl<R, O> BufferedBinaryReader<R, O>
where
    O: ByteOrderSpec,
{
    /// Creates a buffered binary reader with the default buffer capacity.
    #[must_use]
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            input: BufferedInput::new(inner),
            marker: PhantomData,
        }
    }

    /// Creates a buffered binary reader with at least `capacity` bytes.
    #[must_use]
    #[inline]
    pub fn with_capacity(inner: R, capacity: usize) -> Self {
        Self {
            input: BufferedInput::with_capacity(inner, capacity),
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
    ///
    /// The underlying reader may already be positioned past unread bytes held
    /// in this wrapper's internal buffer.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &R {
        self.input.get_ref()
    }

    /// Returns an exclusive reference to the underlying reader.
    ///
    /// Mutating the underlying reader directly can invalidate prefetched bytes
    /// already held in this wrapper's internal buffer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        self.input.get_mut()
    }

    /// Consumes this wrapper and returns the underlying reader.
    ///
    /// Any bytes already prefetched into the internal buffer but not consumed
    /// by codec methods are discarded, matching [`std::io::BufReader`]
    /// `into_inner` semantics.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> R {
        self.input.into_inner()
    }
}

macro_rules! impl_value_read {
    ($order:ty, $method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self) -> Result<$ty> {
            type Codec = BinaryCodec<$ty, $order>;

            const LEN: usize = Codec::REQUIRED_MIN_BUFFER_LEN;
            self.input.read_fixed::<LEN, _, _>(|bytes, index| {
                // SAFETY: `read_fixed` guarantees that `LEN` readable bytes
                // starting at `index` are available in the internal buffer.
                unsafe { Codec::read_unchecked(bytes, index) }
            })
        }
    };
}

macro_rules! impl_for_order {
    ($order:ty) => {
        impl<R> BufferedBinaryReader<R, $order>
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
        }
    };
}

impl_for_order!(BigEndian);
impl_for_order!(LittleEndian);

impl<R, O> Read for BufferedBinaryReader<R, O>
where
    R: Read,
{
    /// Reads bytes from the buffered reader.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.input.read_raw(buffer)
    }
}

impl<R, O> Seek for BufferedBinaryReader<R, O>
where
    R: Read + Seek,
{
    /// Seeks the wrapped reader and discards buffered bytes after success.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.input.seek_raw(position)
    }
}
