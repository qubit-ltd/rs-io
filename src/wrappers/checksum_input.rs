// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
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
///
/// Seeking changes only the source position; it does not reset or otherwise
/// modify the accumulated checksum.
///
/// # Type Parameters
///
/// * `I` - Wrapped byte input type.
/// * `H` - Hasher updated with successfully returned bytes.
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
    ///
    /// # Parameters
    ///
    /// - `inner`: Input that supplies bytes.
    /// - `hasher`: Hasher updated with successfully returned bytes.
    ///
    /// # Returns
    ///
    /// Returns a wrapper with the supplied input and hasher.
    #[inline(always)]
    pub const fn new(inner: I, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    ///
    /// # Returns
    ///
    /// Returns the value reported by the wrapped hasher.
    #[inline(always)]
    #[must_use]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    ///
    /// Reads made through the returned reference bypass the hasher.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped input.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a shared reference to the wrapped hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped hasher without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns mutable access to the wrapped hasher.
    ///
    /// Direct changes become part of the checksum state exposed by this
    /// wrapper.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped hasher.
    #[inline(always)]
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns its input and hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and hasher.
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
    /// # Errors
    ///
    /// Returns an error from the wrapped input, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count. No
    /// bytes are added to the hasher when an error is returned.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    #[inline(always)]
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
