// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

use crate::AsyncInput;

/// Exposes a Qubit byte [`AsyncInput`] as a Tokio [`AsyncRead`].
///
/// # Type Parameters
///
/// - `I`: Qubit byte input type.
#[must_use]
#[repr(transparent)]
pub struct TokioAsyncRead<I> {
    /// Qubit byte input exposed through Tokio.
    inner: I,
}

impl<I> TokioAsyncRead<I> {
    /// Creates a Tokio reader around a Qubit byte input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Qubit byte input to expose.
    ///
    /// # Returns
    ///
    /// Returns a Tokio reader adapter that owns `inner`.
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

impl<I> AsyncRead for TokioAsyncRead<I>
where
    I: AsyncInput<Item = u8>,
{
    /// Polls one Tokio read through the wrapped Qubit input.
    ///
    /// A destination with no remaining capacity completes immediately without
    /// polling `inner`.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Tokio destination buffer to fill.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready, otherwise a ready
    /// success result after advancing `output`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input.
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        self.get_pin_mut()
            .poll_read(cx, output.initialize_unfilled())
            .map(|result| result.map(|read| output.advance(read)))
    }
}
