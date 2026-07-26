// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    hash::Hasher,
    io::{self, SeekFrom},
};

use crate::{Output, Seekable};

/// Byte output wrapper that hashes successfully accepted bytes.
///
/// Seeking changes only the output position; it does not reset or otherwise
/// modify the accumulated checksum.
///
/// # Type Parameters
///
/// * `O` - Wrapped byte output type.
/// * `H` - Hasher updated with successfully accepted bytes.
#[must_use]
#[derive(Debug)]
pub struct ChecksumOutput<O, H> {
    /// Output whose successful bytes are hashed.
    inner: O,
    /// Hasher updated after successful writes.
    hasher: H,
}

impl<O, H> ChecksumOutput<O, H>
where
    H: Hasher,
{
    /// Creates a checksum output around `inner` and `hasher`.
    #[inline(always)]
    pub const fn new(inner: O, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    #[inline(always)]
    #[must_use]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns a shared reference to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    ///
    /// Writes made through the returned reference bypass the hasher.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Returns a shared reference to the wrapped hasher.
    #[inline(always)]
    #[must_use]
    pub const fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns mutable access to the wrapped hasher.
    ///
    /// Direct changes become part of the checksum state exposed by this
    /// wrapper.
    #[inline(always)]
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns its output and hasher.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (O, H) {
        (self.inner, self.hasher)
    }
}

impl<O, H> Output for ChecksumOutput<O, H>
where
    O: Output<Item = u8>,
    H: Hasher,
{
    /// Byte item accepted by this output.
    type Item = u8;

    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Writes bytes and hashes only the successful prefix.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped output, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count. No
    /// bytes are added to the hasher when an error is returned.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `input`.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let written = self.inner.write(&input[index..index + count])?;
        self.hasher.write(&input[index..index + written]);
        Ok(written)
    }

    /// Flushes the wrapped output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<O, H> Seekable for ChecksumOutput<O, H>
where
    O: Seekable,
    H: Hasher,
{
    /// Unit used by the wrapped output for stream positions.
    type Unit = O::Unit;

    /// Seeks the wrapped output without changing the hasher.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
