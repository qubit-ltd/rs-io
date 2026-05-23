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
use std::io::{Read, Result, Seek, SeekFrom};

use crate::ReadExt;
use crate::codec::{BigEndian, ByteOrder, ByteOrderSpec, LittleEndian};
use crate::stream::macros::impl_binary_reader_for_order;

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

impl<R, O> BinaryReader<R, O>
where
    R: Read,
    O: ByteOrderSpec,
{
    #[inline]
    fn read_binary_with<T, const N: usize, F>(&mut self, decode: F) -> Result<T>
    where
        F: FnOnce(&[u8; 16]) -> T,
    {
        // SAFETY: All in-crate callers pass codec-declared lengths that fit
        // the fixed internal buffer.
        unsafe {
            self.inner.read_exact_unchecked(&mut self.buffer, 0, N)?;
        }
        Ok(decode(&self.buffer))
    }
}

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

impl_binary_reader_for_order!(BigEndian);
impl_binary_reader_for_order!(LittleEndian);
