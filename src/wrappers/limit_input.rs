// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use crate::Input;

/// Input wrapper that exposes at most a fixed number of items.
///
/// # Type Parameters
///
/// * `I` - Wrapped input type.
#[must_use]
#[derive(Debug)]
pub struct LimitInput<I> {
    /// Input constrained by this wrapper.
    inner: I,
    /// Number of items still exposed.
    remaining: u64,
}

impl<I> LimitInput<I> {
    /// Creates an input that exposes at most `limit` items.
    ///
    /// # Parameters
    ///
    /// - `inner`: Input constrained by this wrapper.
    /// - `limit`: Maximum number of items the wrapper exposes.
    ///
    /// # Returns
    ///
    /// Returns a wrapper with `limit` remaining items.
    #[inline(always)]
    pub const fn new(inner: I, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the number of items still exposed by this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the remaining number of items this wrapper can expose.
    #[inline(always)]
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
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
    /// Reads made through the returned reference bypass the remaining-item
    /// limit and do not change [`Self::remaining`].
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped input.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> Input for LimitInput<I>
where
    I: Input,
{
    /// Item type exposed by the limited input.
    type Item = I::Item;

    /// Returns the wrapped input's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns whether the wrapped input reports itself as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads only the still-exposed prefix of the requested item range.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items requested.
    ///
    /// # Returns
    ///
    /// Returns the number of items read within the remaining limit.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped input, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count. The
    /// remaining limit is unchanged when an error is returned.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be a valid range in `output`.
    #[inline]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        if self.remaining == 0 || count == 0 {
            return Ok(0);
        }
        let requested = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(count);
        let read = self.inner.read(&mut output[index..index + requested])?;
        self.remaining -= u64::try_from(read).unwrap_or(u64::MAX);
        Ok(read)
    }
}
