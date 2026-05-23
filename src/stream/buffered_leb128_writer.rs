/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Result, Seek, SeekFrom, Write};

use crate::codec::{Leb128Codec, NonStrict};
use crate::stream::BufferedOutput;

/// Buffered writer for canonical LEB128 integers.
///
/// Values are encoded directly into the internal output buffer and flushed to
/// the wrapped writer in larger chunks.
pub struct BufferedLeb128Writer<W> {
    output: BufferedOutput<W>,
}

impl<W> BufferedLeb128Writer<W> {
    /// Creates a buffered LEB128 writer with the default buffer capacity.
    #[must_use]
    #[inline]
    pub fn new(inner: W) -> Self {
        Self {
            output: BufferedOutput::new(inner),
        }
    }

    /// Creates a buffered LEB128 writer with at least `capacity` bytes.
    #[must_use]
    #[inline]
    pub fn with_capacity(inner: W, capacity: usize) -> Self {
        Self {
            output: BufferedOutput::with_capacity(inner, capacity),
        }
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

impl<W> BufferedLeb128Writer<W>
where
    W: Write,
{
    /// Flushes pending bytes and returns the underlying writer.
    #[inline]
    pub fn into_inner(self) -> Result<W> {
        self.output.into_inner()
    }

    /// Writes a UTF-8 string prefixed by an unsigned LEB128 byte length.
    #[inline]
    pub fn write_utf8_string(&mut self, value: &str) -> Result<()> {
        self.write_usize(value.len())?;
        self.output.write_all_buffered(value.as_bytes())
    }
}

macro_rules! impl_write_value {
    ($method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self, value: $ty) -> Result<()> {
            type Codec = Leb128Codec<$ty, NonStrict>;

            self.output.write_encoded(
                Codec::REQUIRED_MIN_BUFFER_LEN,
                value,
                |bytes, index, value| {
                    // SAFETY: `write_encoded` guarantees enough writable bytes
                    // for the codec-declared maximum encoded width.
                    unsafe { Codec::write_unchecked(bytes, index, value) }
                },
            )
        }
    };
}

impl<W> BufferedLeb128Writer<W>
where
    W: Write,
{
    impl_write_value!(write_u8, u8, "Writes an unsigned LEB128 `u8`.");
    impl_write_value!(write_u16, u16, "Writes an unsigned LEB128 `u16`.");
    impl_write_value!(write_u32, u32, "Writes an unsigned LEB128 `u32`.");
    impl_write_value!(write_u64, u64, "Writes an unsigned LEB128 `u64`.");
    impl_write_value!(write_u128, u128, "Writes an unsigned LEB128 `u128`.");
    impl_write_value!(write_usize, usize, "Writes an unsigned LEB128 `usize`.");
    impl_write_value!(write_i8, i8, "Writes a signed LEB128 `i8`.");
    impl_write_value!(write_i16, i16, "Writes a signed LEB128 `i16`.");
    impl_write_value!(write_i32, i32, "Writes a signed LEB128 `i32`.");
    impl_write_value!(write_i64, i64, "Writes a signed LEB128 `i64`.");
    impl_write_value!(write_i128, i128, "Writes a signed LEB128 `i128`.");
    impl_write_value!(write_isize, isize, "Writes a signed LEB128 `isize`.");
}

impl<W> Write for BufferedLeb128Writer<W>
where
    W: Write,
{
    /// Writes bytes through the internal buffer.
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.output.write_raw(buffer)
    }

    /// Writes all bytes through the internal buffer.
    #[inline]
    fn write_all(&mut self, buffer: &[u8]) -> Result<()> {
        self.output.write_all_buffered(buffer)
    }

    /// Flushes the internal buffer and then the wrapped writer.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.output.flush_all()
    }
}

impl<W> Seek for BufferedLeb128Writer<W>
where
    W: Write + Seek,
{
    /// Flushes pending bytes before seeking the wrapped writer.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.output.seek_raw(position)
    }
}
