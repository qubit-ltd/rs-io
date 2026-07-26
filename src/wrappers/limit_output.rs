// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::io;

use crate::Output;

/// Output wrapper that accepts at most a fixed number of items.
#[must_use]
#[derive(Debug)]
pub struct LimitOutput<O> {
    /// Output constrained by this wrapper.
    inner: O,
    /// Number of items still accepted.
    remaining: u64,
}

impl<O> LimitOutput<O> {
    /// Creates an output that accepts at most `limit` items.
    #[inline(always)]
    pub const fn new(inner: O, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the number of items still accepted by this wrapper.
    #[inline(always)]
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns a shared reference to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
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

impl<O> Output for LimitOutput<O>
where
    O: Output,
{
    /// Item type accepted by the limited output.
    type Item = O::Item;

    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Writes only the still-accepted prefix of the requested item range.
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
        if self.remaining == 0 || count == 0 {
            return Ok(0);
        }
        let requested = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(count);
        let written = self.inner.write(&input[index..index + requested])?;
        self.remaining -= u64::try_from(written).unwrap_or(u64::MAX);
        Ok(written)
    }

    /// Flushes the wrapped output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
