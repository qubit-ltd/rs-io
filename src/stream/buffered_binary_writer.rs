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
use std::io::{Result, Seek, SeekFrom, Write};

use crate::codec::{BigEndian, BinaryCodec, ByteOrder, ByteOrderSpec, LittleEndian};
use crate::stream::BufferedOutput;

/// Buffered writer for fixed-width binary values.
///
/// Scalar writes encode directly into the internal output buffer and flush that
/// buffer to the wrapped writer only when it becomes full or when explicitly
/// flushed.
pub struct BufferedBinaryWriter<W, O = BigEndian> {
    output: BufferedOutput<W>,
    marker: PhantomData<fn() -> O>,
}

impl<W, O> BufferedBinaryWriter<W, O>
where
    O: ByteOrderSpec,
{
    /// Creates a buffered binary writer with the default buffer capacity.
    #[must_use]
    #[inline]
    pub fn new(inner: W) -> Self {
        Self {
            output: BufferedOutput::new(inner),
            marker: PhantomData,
        }
    }

    /// Creates a buffered binary writer with at least `capacity` bytes.
    #[must_use]
    #[inline]
    pub fn with_capacity(inner: W, capacity: usize) -> Self {
        Self {
            output: BufferedOutput::with_capacity(inner, capacity),
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
        self.output.get_ref()
    }

    /// Returns an exclusive reference to the underlying writer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        self.output.get_mut()
    }
}

impl<W, O> BufferedBinaryWriter<W, O>
where
    W: Write,
    O: ByteOrderSpec,
{
    /// Flushes pending bytes and returns the underlying writer.
    #[inline]
    pub fn into_inner(self) -> Result<W> {
        self.output.into_inner()
    }
}

macro_rules! impl_value_write {
    ($order:ty, $method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self, value: $ty) -> Result<()> {
            type Codec = BinaryCodec<$ty, $order>;

            const LEN: usize = Codec::REQUIRED_MIN_BUFFER_LEN;
            self.output
                .write_encoded(LEN, value, |bytes, index, value| {
                    // SAFETY: `write_encoded` guarantees that `LEN` writable bytes
                    // starting at `index` are available in the internal buffer.
                    unsafe {
                        Codec::write_unchecked(bytes, index, value);
                    }
                    LEN
                })
        }
    };
}

macro_rules! impl_for_order {
    ($order:ty) => {
        impl<W> BufferedBinaryWriter<W, $order>
        where
            W: Write,
        {
            impl_value_write!($order, write_u8, u8, "Writes an unsigned 8-bit integer.");
            impl_value_write!($order, write_i8, i8, "Writes a signed 8-bit integer.");
            impl_value_write!($order, write_u16, u16, "Writes an unsigned 16-bit integer.");
            impl_value_write!($order, write_u32, u32, "Writes an unsigned 32-bit integer.");
            impl_value_write!($order, write_u64, u64, "Writes an unsigned 64-bit integer.");
            impl_value_write!(
                $order,
                write_u128,
                u128,
                "Writes an unsigned 128-bit integer."
            );
            impl_value_write!($order, write_i16, i16, "Writes a signed 16-bit integer.");
            impl_value_write!($order, write_i32, i32, "Writes a signed 32-bit integer.");
            impl_value_write!($order, write_i64, i64, "Writes a signed 64-bit integer.");
            impl_value_write!($order, write_i128, i128, "Writes a signed 128-bit integer.");
            impl_value_write!($order, write_f32, f32, "Writes a 32-bit float.");
            impl_value_write!($order, write_f64, f64, "Writes a 64-bit float.");
        }
    };
}

impl_for_order!(BigEndian);
impl_for_order!(LittleEndian);

impl<W, O> Write for BufferedBinaryWriter<W, O>
where
    W: Write,
{
    /// Writes bytes through the internal buffer.
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.output.write_raw(buffer)
    }

    /// Flushes the internal buffer and then the wrapped writer.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.output.flush_all()
    }
}

impl<W, O> Seek for BufferedBinaryWriter<W, O>
where
    W: Write + Seek,
{
    /// Flushes pending bytes before seeking the wrapped writer.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.output.seek_raw(position)
    }
}
