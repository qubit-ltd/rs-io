// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_io::AsyncRead;

use crate::AsyncInput;

/// Exposes a Qubit byte [`AsyncInput`] as a futures-io [`AsyncRead`].
///
/// # Type Parameters
///
/// - `I`: Qubit byte input type.
#[must_use]
#[repr(transparent)]
pub struct FuturesAsyncRead<I> {
    /// Qubit byte input exposed through futures-io.
    inner: I,
}

impl<I> FuturesAsyncRead<I> {
    /// Creates a futures-io reader around a Qubit byte input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Qubit byte input to expose.
    ///
    /// # Returns
    ///
    /// Returns a futures-io reader adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit input.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit input with mutable access.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped input.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped Qubit input without
    /// moving it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut I> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the owned Qubit input.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> AsyncRead for FuturesAsyncRead<I>
where
    I: AsyncInput<Item = u8>,
{
    /// Polls one futures-io read through the wrapped Qubit input.
    ///
    /// An empty destination completes immediately without polling `inner`.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Destination byte slice.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready. A ready result
    /// contains the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input.
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if output.is_empty() {
            return Poll::Ready(Ok(0));
        }
        self.get_pin_mut().poll_read(cx, output)
    }
}
