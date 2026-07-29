// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
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
///
/// The counter saturates at [`u64::MAX`].
///
/// # Type Parameters
///
/// * `O` - Wrapped output type.
#[must_use]
#[derive(Debug)]
pub struct CountingOutput<O> {
    /// Output whose successful transfers are counted.
    inner: O,
    /// Saturating count of accepted items.
    items_written: u64,
}

impl<O> CountingOutput<O> {
    /// Creates a counting output around `inner` with a zero item count.
    ///
    /// # Parameters
    ///
    /// - `inner`: Output whose successful transfers are counted.
    ///
    /// # Returns
    ///
    /// Returns a wrapper with its item count initialized to zero.
    #[inline(always)]
    pub const fn new(inner: O) -> Self {
        Self {
            inner,
            items_written: 0,
        }
    }

    /// Returns the saturating number of items successfully accepted through
    /// this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the saturating count of successful item transfers.
    #[inline(always)]
    #[must_use]
    pub const fn items_written(&self) -> u64 {
        self.items_written
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    ///
    /// Writes through the returned reference do not affect this wrapper's
    /// counter.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output.
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
    ///
    /// # Returns
    ///
    /// Returns the saturating number of successfully accepted bytes.
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
    ///
    /// # Returns
    ///
    /// Returns whether the wrapped output reports itself as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Writes items and counts only a successful, validated result.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns the number of items written and added to the counter.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped output, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be a valid range in `input`.
    #[inline]
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
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the wrapped output is flushed.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output.
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
    ///
    /// # Parameters
    ///
    /// - `position`: Target stream position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute stream position.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
