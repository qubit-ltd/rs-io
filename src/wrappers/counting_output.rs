// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::io::{
    self,
    SeekFrom,
};

use crate::{
    Output,
    Seekable,
};

/// Output wrapper that counts successfully accepted items.
#[must_use]
#[derive(Debug)]
pub struct CountingOutput<O> {
    /// Output whose successful transfers are counted.
    inner: O,
    /// Saturating count of accepted items.
    items_written: u64,
}

impl<O> CountingOutput<O> {
    /// Creates a counting output around `inner`.
    #[inline(always)]
    pub const fn new(inner: O) -> Self {
        Self {
            inner,
            items_written: 0,
        }
    }

    /// Returns the number of items successfully accepted through this wrapper.
    #[inline(always)]
    #[must_use]
    pub const fn items_written(&self) -> u64 {
        self.items_written
    }

    /// Returns a shared reference to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    ///
    /// Writes through the returned reference do not affect this wrapper's
    /// counter.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> CountingOutput<O>
where
    O: Output<Item = u8>,
{
    /// Returns the successfully accepted byte count.
    #[inline(always)]
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.items_written
    }
}

impl<O> Output for CountingOutput<O>
where
    O: Output,
{
    /// Item type accepted by the wrapped output.
    type Item = O::Item;

    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Writes items and counts only a successful, validated result.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be a valid range in `input`.
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let source = &input[index..index + count];
        let written = self.inner.write(source)?;
        let written_u64 = u64::try_from(written).unwrap_or(u64::MAX);
        self.items_written = self.items_written.saturating_add(written_u64);
        Ok(written)
    }

    /// Flushes the wrapped output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<O> Seekable for CountingOutput<O>
where
    O: Seekable,
{
    /// Unit used by the wrapped output for stream positions.
    type Unit = O::Unit;

    /// Seeks the wrapped output without changing the item count.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
