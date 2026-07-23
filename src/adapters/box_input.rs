// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use crate::Input;

/// Adapts an owned boxed input, including a boxed input trait object.
///
/// # Type Parameters
///
/// - `I`: The boxed input type, which may be unsized.
#[repr(transparent)]
pub struct BoxInput<I>
where
    I: Input + ?Sized,
{
    inner: Box<I>,
}

impl<I> BoxInput<I>
where
    I: Input + ?Sized,
{
    /// Creates an input adapter around `inner`.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: Box<I>) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the boxed input.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &I {
        self.inner.as_ref()
    }

    /// Returns mutable access to the boxed input.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut I {
        self.inner.as_mut()
    }

    /// Consumes this adapter and returns its boxed input.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> Box<I> {
        self.inner
    }
}

impl<I> Input for BoxInput<I>
where
    I: Input + ?Sized,
{
    type Item = I::Item;

    /// Returns the wrapped input's buffering capability.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked read to the boxed input.
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

    /// Forwards a checked read to the boxed input.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read(output)
    }

    /// Forwards an unchecked complete read to the boxed input.
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

    /// Forwards a complete read to the boxed input.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read_fully(output)
    }
}
