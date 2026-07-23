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
/// - `I`: The borrowed input type, which may be unsized.
#[repr(transparent)]
pub struct InputRef<'a, I>
where
    I: Input + ?Sized,
{
    inner: &'a mut I,
}

impl<'a, I> InputRef<'a, I>
where
    I: Input + ?Sized,
{
    /// Creates an input adapter that borrows `inner` mutably.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: &'a mut I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the borrowed input.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &I {
        self.inner
    }

    /// Returns mutable access to the borrowed input.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut I {
        self.inner
    }

    /// Consumes this adapter and returns its mutable input borrow.
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
    type Item = I::Item;

    /// Returns the wrapped input's buffering capability.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked read to the wrapped input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.read_unchecked(output, index, count) }
    }

    /// Forwards a checked read to the wrapped input.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read(output)
    }

    /// Forwards an unchecked complete read to the wrapped input.
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
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read_fully(output)
    }
}
