// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use crate::Output;

/// Adapts an owned boxed output, including a boxed output trait object.
///
/// # Type Parameters
///
/// - `O`: The boxed output type, which may be unsized.
#[repr(transparent)]
pub struct BoxOutput<O>
where
    O: Output + ?Sized,
{
    inner: Box<O>,
}

impl<O> BoxOutput<O>
where
    O: Output + ?Sized,
{
    /// Creates an output adapter around `inner`.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: Box<O>) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the boxed output.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &O {
        self.inner.as_ref()
    }

    /// Returns mutable access to the boxed output.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut O {
        self.inner.as_mut()
    }

    /// Consumes this adapter and returns its boxed output.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> Box<O> {
        self.inner
    }
}

impl<O> Output for BoxOutput<O>
where
    O: Output + ?Sized,
{
    type Item = O::Item;

    /// Returns the wrapped output's buffering capability.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked write to the boxed output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.write_unchecked(input, index, count) }
    }

    /// Forwards a checked write to the boxed output.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> io::Result<usize> {
        self.inner.write(input)
    }

    /// Forwards an unchecked complete write to the boxed output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline(always)]
    unsafe fn write_fully_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<()> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.write_fully_unchecked(input, index, count) }
    }

    /// Forwards a complete write to the boxed output.
    #[inline(always)]
    fn write_fully(&mut self, input: &[Self::Item]) -> io::Result<()> {
        self.inner.write_fully(input)
    }

    /// Flushes the boxed output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
