// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use crate::Input;

/// Adapts a mutable borrowed input where an owned [`Input`] value is required.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the mutable input borrow.
/// - `I`: The borrowed input type, which may be unsized.
#[must_use]
#[repr(transparent)]
pub struct InputRef<'a, I>
where
    I: Input + ?Sized,
{
    /// Mutable input borrow being adapted.
    inner: &'a mut I,
}

impl<'a, I> InputRef<'a, I>
where
    I: Input + ?Sized,
{
    /// Creates an input adapter that borrows `inner` mutably.
    ///
    /// # Parameters
    ///
    /// - `inner`: Mutable input borrow to adapt.
    ///
    /// # Returns
    ///
    /// Returns an adapter that retains the borrow for `'a`.
    #[inline(always)]
    pub const fn new(inner: &'a mut I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the borrowed input.
    ///
    /// # Returns
    ///
    /// Returns the borrowed input with shared access.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &I {
        self.inner
    }

    /// Returns mutable access to the borrowed input.
    ///
    /// # Returns
    ///
    /// Returns the borrowed input with mutable access.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut I {
        self.inner
    }

    /// Consumes this adapter and returns its mutable input borrow.
    ///
    /// # Returns
    ///
    /// Returns the original mutable borrow.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> &'a mut I {
        self.inner
    }
}

impl<I> Input for InputRef<'_, I>
where
    I: Input + ?Sized,
{
    /// Item type read from the borrowed input.
    type Item = I::Item;

    /// Returns the wrapped input's buffering capability.
    ///
    /// # Returns
    ///
    /// Returns `true` when the borrowed input is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked read to the wrapped input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline(always)]
    unsafe fn read_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> io::Result<usize> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.read_unchecked(output, index, count) }
    }

    /// Forwards a checked read to the wrapped input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed input.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read(output)
    }

    /// Forwards an unchecked complete read to the wrapped input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    /// - `index`: Starting destination index.
    /// - `count`: Number of items requested.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the range was filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline(always)]
    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.read_fully_unchecked(output, index, count) }
    }

    /// Forwards a complete read to the wrapped input.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the slice was filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed input.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read_fully(output)
    }
}
