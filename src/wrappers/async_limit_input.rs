// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncInput;

/// Asynchronous input that exposes at most a fixed number of items.
///
/// # Type Parameters
///
/// - `I`: Wrapped asynchronous input type.
#[must_use]
#[derive(Debug)]
pub struct AsyncLimitInput<I> {
    /// Input constrained by this wrapper.
    inner: I,
    /// Number of items still exposed.
    remaining: u64,
}

impl<I> AsyncLimitInput<I> {
    /// Creates a limited asynchronous input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous input to wrap.
    /// - `limit`: Maximum item count exposed by this wrapper.
    ///
    /// # Returns
    ///
    /// Returns an input with `limit` items remaining.
    #[inline(always)]
    pub const fn new(inner: I, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the remaining exposed item count.
    ///
    /// # Returns
    ///
    /// Returns zero after the configured limit has been consumed.
    #[inline(always)]
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// Reads performed directly on the returned input bypass this wrapper's
    /// remaining-item limit.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous input without changing its position.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> AsyncInput for AsyncLimitInput<I>
where
    I: AsyncInput,
{
    /// Item type exposed by the limited input.
    type Item = I::Item;

    /// Preserves the wrapped input's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a read bounded by the remaining item count.
    ///
    /// The method completes with zero items without polling `inner` when the
    /// limit is exhausted or `count` is zero.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items requested.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready. A ready success
    /// contains the number of items read within the remaining limit.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input without consuming the
    /// remaining limit.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
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
        let destination = &mut output[index..index + requested];
        // SAFETY: The pinned wrapper never moves `inner`.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_read(cx, destination) {
            Poll::Ready(Ok(read)) => {
                this.remaining -= read as u64;
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
