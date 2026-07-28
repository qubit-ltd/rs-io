// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{self, SeekFrom};

use crate::{Input, Seekable};

/// Input wrapper that counts successfully returned items.
///
/// Failed reads and invalid counts reported by the wrapped input do not change
/// the counter. The counter saturates at [`u64::MAX`].
///
/// # Type Parameters
///
/// * `I` - Wrapped input type.
#[must_use]
#[derive(Debug)]
pub struct CountingInput<I> {
    /// Input whose successful transfers are counted.
    inner: I,
    /// Saturating count of returned items.
    items_read: u64,
}

impl<I> CountingInput<I> {
    /// Creates a counting input around `inner` with a zero item count.
    #[inline(always)]
    pub const fn new(inner: I) -> Self {
        Self {
            inner,
            items_read: 0,
        }
    }

    /// Returns the saturating number of items successfully returned through
    /// this wrapper.
    #[inline(always)]
    #[must_use]
    pub const fn items_read(&self) -> u64 {
        self.items_read
    }

    /// Returns a shared reference to the wrapped input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    ///
    /// Reads through the returned reference do not affect this wrapper's
    /// counter.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped input.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> CountingInput<I>
where
    I: Input<Item = u8>,
{
    /// Returns the successfully returned byte count.
    #[inline(always)]
    #[must_use]
    pub const fn bytes_read(&self) -> u64 {
        self.items_read
    }
}

impl<I> Input for CountingInput<I>
where
    I: Input,
{
    /// Item type returned by the wrapped input.
    type Item = I::Item;

    /// Returns the wrapped input's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads items and counts only a successful, validated result.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped input, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be a valid range in `output`.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let destination = &mut output[index..index + count];
        let read = self.inner.read(destination)?;
        let read_u64 = u64::try_from(read).unwrap_or(u64::MAX);
        self.items_read = self.items_read.saturating_add(read_u64);
        Ok(read)
    }
}

impl<I> Seekable for CountingInput<I>
where
    I: Seekable,
{
    /// Unit used by the wrapped input for stream positions.
    type Unit = I::Unit;

    /// Seeks the wrapped input without changing the item count.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
