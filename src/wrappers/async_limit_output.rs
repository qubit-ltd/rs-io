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
    task::{
        Context,
        Poll,
    },
};

use crate::{
    AsyncClose,
    AsyncOutput,
    traits::validate_async_error,
};

/// Asynchronous output that accepts at most a fixed number of items.
#[derive(Debug)]
pub struct AsyncLimitOutput<O> {
    inner: O,
    remaining: u64,
}

impl<O> AsyncClose for AsyncLimitOutput<O>
where
    O: AsyncClose,
{
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_close(cx)
            .map(|result| result.map_err(validate_async_error))
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
    #[must_use]
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
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
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
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous output.
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> AsyncOutput for AsyncLimitOutput<O>
where
    O: AsyncOutput,
{
    type Item = O::Item;

    /// Preserves the wrapped output's buffering declaration.
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write bounded by the remaining item count.
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
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(validate_async_error))
    }
}
