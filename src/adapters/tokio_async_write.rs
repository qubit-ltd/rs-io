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

use tokio::io::AsyncWrite;

use crate::AsyncOutput;

/// Exposes a Qubit byte [`AsyncOutput`] as a Tokio [`AsyncWrite`].
#[repr(transparent)]
pub struct TokioAsyncWrite<O> {
    inner: O,
}

impl<O> TokioAsyncWrite<O> {
    /// Creates a Tokio writer around a Qubit byte output.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: O) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut O> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> AsyncWrite for TokioAsyncWrite<O>
where
    O: AsyncOutput<Item = u8>,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_pin_mut().poll_write(cx, input)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.get_pin_mut().poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.get_pin_mut().poll_flush(cx)
    }
}
