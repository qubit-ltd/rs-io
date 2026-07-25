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
#[must_use]
#[repr(transparent)]
pub struct BoxInput<I>
where
    I: Input + ?Sized,
{
    /// Owned input being adapted.
    inner: Box<I>,
}

impl<I> BoxInput<I>
where
    I: Input + ?Sized,
{
    /// Creates an input adapter around `inner`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Boxed input to adapt.
    ///
    /// # Returns
    ///
    /// Returns an adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: Box<I>) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the boxed input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &I {
        self.inner.as_ref()
    }

    /// Returns mutable access to the boxed input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input with mutable access.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut I {
        self.inner.as_mut()
    }

    /// Consumes this adapter and returns its boxed input.
    ///
    /// # Returns
    ///
    /// Returns the owned boxed input.
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
    /// Item type read from the wrapped input.
    type Item = I::Item;

    /// Returns the wrapped input's buffering capability.
    ///
    /// # Returns
    ///
    /// Returns `true` when the wrapped input is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked read to the boxed input.
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
    /// Returns an error reported by the wrapped input.
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
    /// Returns an error reported by the wrapped input.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read(output)
    }

    /// Forwards an unchecked complete read to the boxed input.
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
    /// Returns an error reported by the wrapped input.
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
    /// Returns an error reported by the wrapped input.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> io::Result<usize> {
        self.inner.read_fully(output)
    }
}
