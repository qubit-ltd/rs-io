// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{AsyncClose, AsyncOutput, traits::normalize_async_error};

/// Asynchronous output that accepts at most a fixed number of items.
///
/// # Type Parameters
///
/// - `O`: Wrapped asynchronous output type.
#[must_use]
#[derive(Debug)]
pub struct AsyncLimitOutput<O> {
    /// Output constrained by this wrapper.
    inner: O,
    /// Number of items still accepted.
    remaining: u64,
}

impl<O> AsyncClose for AsyncLimitOutput<O>
where
    O: AsyncClose,
{
    /// Polls closing through the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while closing is incomplete, otherwise a
    /// ready success result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output. Invalid asynchronous
    /// error kinds are normalized to [`io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}

impl<O> AsyncLimitOutput<O> {
    /// Creates a limited asynchronous output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous output to wrap.
    /// - `limit`: Maximum item count accepted by this wrapper.
    ///
    /// # Returns
    ///
    /// Returns an output with `limit` items remaining.
    #[inline(always)]
    pub const fn new(inner: O, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the remaining accepted item count.
    ///
    /// # Returns
    ///
    /// Returns zero after the configured limit has been consumed.
    #[inline(always)]
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// Writes performed directly on the returned output bypass this wrapper's
    /// remaining-item limit.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous output.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> AsyncOutput for AsyncLimitOutput<O>
where
    O: AsyncOutput,
{
    /// Item type accepted by the limited output.
    type Item = O::Item;

    /// Preserves the wrapped output's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write bounded by the remaining item count.
    ///
    /// The method completes with zero items without polling `inner` when the
    /// limit is exhausted or `count` is zero.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items offered.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the output is not ready. A ready success
    /// contains the number of items accepted within the remaining limit.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped output without consuming
    /// the remaining limit.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if this.remaining == 0 || count == 0 {
            return Poll::Ready(Ok(0));
        }
        let requested = usize::try_from(this.remaining)
            .unwrap_or(usize::MAX)
            .min(count);
        let source = &input[index..index + requested];
        // SAFETY: The pinned wrapper never moves `inner`.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_write(cx, source) {
            Poll::Ready(Ok(written)) => {
                this.remaining -= written as u64;
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Polls the wrapped output's flush operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while flushing is incomplete, otherwise a
    /// ready success result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output. Invalid asynchronous
    /// error kinds are normalized to [`io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
