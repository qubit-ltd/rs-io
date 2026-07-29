// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use crate::AsyncInput;

/// Adapts an owned pinned boxed asynchronous input, including a trait object.
///
/// # Type Parameters
///
/// - `I`: The boxed asynchronous input type, which may be unsized and
///   non-`Unpin`.
#[must_use]
#[repr(transparent)]
pub struct BoxAsyncInput<I>
where
    I: AsyncInput + ?Sized,
{
    /// Pinned asynchronous input owned by this adapter.
    inner: Pin<Box<I>>,
}

impl<I> BoxAsyncInput<I>
where
    I: AsyncInput + ?Sized,
{
    /// Creates an asynchronous input adapter around `inner`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Boxed asynchronous input to pin and adapt.
    ///
    /// # Returns
    ///
    /// Returns an adapter that owns and pins `inner`.
    #[inline(always)]
    pub fn new(inner: Box<I>) -> Self {
        Self {
            inner: Box::into_pin(inner),
        }
    }

    /// Returns a shared reference to the boxed asynchronous input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &I {
        self.inner.as_ref().get_ref()
    }

    /// Projects a pinned adapter to its pinned boxed asynchronous input.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped input without moving
    /// it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut I> {
        // SAFETY: Accessing the field through a pinned mutable reference does
        // not move `inner`, whose allocation remains pinned.
        let this = unsafe { self.get_unchecked_mut() };
        this.inner.as_mut()
    }

    /// Consumes this adapter and returns its pinned boxed asynchronous input.
    ///
    /// # Returns
    ///
    /// Returns the pinned boxed input so a potentially non-`Unpin` value stays
    /// pinned after extraction.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> Pin<Box<I>> {
        self.inner
    }
}

impl<I> AsyncInput for BoxAsyncInput<I>
where
    I: AsyncInput + ?Sized,
{
    /// Item type read from the wrapped asynchronous input.
    type Item = I::Item;

    /// Returns the wrapped input's buffering capability.
    ///
    /// # Returns
    ///
    /// Returns `true` when the wrapped input is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.get_ref().is_buffered()
    }

    /// Forwards an unchecked asynchronous read to the pinned boxed input.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns a pending state or the wrapped input's ready read result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline(always)]
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe {
            self.get_pin_mut()
                .poll_read_unchecked(cx, output, index, count)
        }
    }
}
