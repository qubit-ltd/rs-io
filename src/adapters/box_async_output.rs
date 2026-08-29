// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncClose;
use crate::AsyncOutput;

/// Adapts an owned pinned boxed asynchronous output, including a trait object.
///
/// # Type Parameters
///
/// - `O`: The boxed asynchronous output type, which may be unsized and
///   non-`Unpin`.
#[must_use]
#[repr(transparent)]
pub struct BoxAsyncOutput<O>
where
    O: AsyncOutput + ?Sized,
{
    /// Pinned asynchronous output owned by this adapter.
    inner: Pin<Box<O>>,
}

impl<O> BoxAsyncOutput<O>
where
    O: AsyncOutput + ?Sized,
{
    /// Creates an asynchronous output adapter around `inner`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Boxed asynchronous output to pin and adapt.
    ///
    /// # Returns
    ///
    /// Returns an adapter that owns and pins `inner`.
    #[inline(always)]
    pub fn new(inner: Box<O>) -> Self {
        Self {
            inner: Box::into_pin(inner),
        }
    }

    /// Returns a shared reference to the boxed asynchronous output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &O {
        self.inner.as_ref().get_ref()
    }

    /// Projects a pinned adapter to its pinned boxed asynchronous output.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped output without moving
    /// it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut O> {
        // SAFETY: Accessing the field through a pinned mutable reference does
        // not move `inner`, whose allocation remains pinned.
        let this = unsafe { self.get_unchecked_mut() };
        this.inner.as_mut()
    }

    /// Consumes this adapter and returns its pinned boxed asynchronous output.
    ///
    /// # Returns
    ///
    /// Returns the pinned boxed output so a potentially non-`Unpin` value stays
    /// pinned after extraction.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> Pin<Box<O>> {
        self.inner
    }
}

impl<O> AsyncOutput for BoxAsyncOutput<O>
where
    O: AsyncOutput + ?Sized,
{
    /// Item type written to the wrapped asynchronous output.
    type Item = O::Item;

    /// Returns the wrapped output's buffering capability.
    ///
    /// # Returns
    ///
    /// Returns `true` when the wrapped output is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.get_ref().is_buffered()
    }

    /// Forwards an unchecked asynchronous write to the pinned boxed output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns a pending state or the wrapped output's ready write result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline(always)]
    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.get_pin_mut().poll_write_unchecked(cx, input, index, count) }
    }

    /// Forwards an asynchronous flush to the pinned boxed output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns a pending state or the wrapped output's ready flush result.
    ///
    /// # Errors
    ///
    /// Returns an error reported while flushing the wrapped output.
    #[inline(always)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_pin_mut().poll_flush(cx)
    }
}

impl<O> AsyncClose for BoxAsyncOutput<O>
where
    O: AsyncClose + ?Sized,
{
    /// Forwards an asynchronous close to the pinned boxed output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns a pending state or the wrapped output's ready close result.
    ///
    /// # Errors
    ///
    /// Returns an error reported while closing the wrapped output.
    #[inline(always)]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_pin_mut().poll_close(cx)
    }
}
