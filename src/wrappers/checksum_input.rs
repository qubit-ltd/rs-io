// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::{
    hash::Hasher,
    io::{
        self,
        SeekFrom,
    },
};

use crate::{
    Input,
    Seekable,
};

/// Byte input wrapper that hashes successfully returned bytes.
#[must_use]
#[derive(Debug)]
pub struct ChecksumInput<I, H> {
    /// Input whose successful bytes are hashed.
    inner: I,
    /// Hasher updated after successful reads.
    hasher: H,
}

impl<I, H> ChecksumInput<I, H>
where
    H: Hasher,
{
    /// Creates a checksum input around `inner` and `hasher`.
    #[inline(always)]
    pub const fn new(inner: I, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    #[inline(always)]
    #[must_use]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns a shared reference to the wrapped input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a shared reference to the wrapped hasher.
    #[inline(always)]
    #[must_use]
    pub const fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns mutable access to the wrapped hasher.
    #[inline(always)]
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns its input and hasher.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, H) {
        (self.inner, self.hasher)
    }
}

impl<I, H> Input for ChecksumInput<I, H>
where
    I: Input<Item = u8>,
    H: Hasher,
{
    /// Byte item returned by this input.
    type Item = u8;

    /// Returns the wrapped input's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads bytes and hashes only the successful prefix.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = self.inner.read(&mut output[index..index + count])?;
        self.hasher.write(&output[index..index + read]);
        Ok(read)
    }
}

impl<I, H> Seekable for ChecksumInput<I, H>
where
    I: Seekable,
    H: Hasher,
{
    /// Unit used by the wrapped input for stream positions.
    type Unit = I::Unit;

    /// Seeks the wrapped input without changing the hasher.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
