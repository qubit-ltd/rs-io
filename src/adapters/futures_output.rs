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

use futures_io::AsyncWrite;

use crate::{
    AsyncClose,
    AsyncOutput,
    UncheckedSlice,
    traits::validate_async_error,
};

/// Adapts a futures-io [`AsyncWrite`] value to Qubit's [`AsyncOutput`].
#[repr(transparent)]
pub struct FuturesOutput<T> {
    inner: T,
}

impl<T> FuturesOutput<T> {
    /// Creates an adapter around a futures-io writer.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped writer.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped writer.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped writer.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> AsyncOutput for FuturesOutput<T>
where
    T: AsyncWrite,
{
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), index, count),
            "unchecked futures-io write range exceeds input buffer"
        );
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        // SAFETY: The caller guarantees that the source range is valid.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        AsyncWrite::poll_write(self.get_pin_mut(), cx, source)
            .map(|result| result.map_err(validate_async_error))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_flush(self.get_pin_mut(), cx)
            .map(|result| result.map_err(validate_async_error))
    }
}

impl<T> AsyncClose for FuturesOutput<T>
where
    T: AsyncWrite,
{
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_close(self.get_pin_mut(), cx)
            .map(|result| result.map_err(validate_async_error))
    }
}
