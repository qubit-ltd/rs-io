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
use std::io::{Error, ErrorKind, Result, Seek, SeekFrom, Write};

use crate::WriteExt;
use crate::codec::{BigEndian, ByteOrder, ByteOrderSpec, LittleEndian};
use crate::stream::macros::impl_binary_writer_for_order;

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

    #[inline]
    fn write_binary<T, const N: usize, F>(&mut self, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8; 16], T),
    {
        encode(&mut self.buffer, value);
        // SAFETY: All in-crate callers pass codec-declared lengths that fit
        // the fixed internal buffer.
        unsafe { self.inner.write_all_unchecked(&self.buffer, 0, N) }
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
pub(crate) fn checked_u16_len(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u16 length"),
        )
    })
}

#[inline]
pub(crate) fn checked_u32_len(len: usize) -> Result<u32> {
    if len > u32::MAX as usize {
        Err(Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u32 length"),
        ))
    } else {
        Ok(len as u32)
    }
}
