// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncInput;

/// Future for one [`AsyncInput`] read operation.
///
/// # Panics
///
/// [`Future::poll`] panics when called again after this future has returned
/// [`Poll::Ready`].
///
/// # Type Parameters
///
/// - `'a`: Shared lifetime of the input borrow and destination slice.
/// - `I`: Asynchronous input type.
#[must_use = "futures do nothing unless polled"]
pub struct ReadFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Input being read.
    input: Pin<&'a mut I>,
    /// Destination for the read.
    output: &'a mut [I::Item],
    /// Whether the read operation has completed.
    completed: bool,
}

impl<'a, I> ReadFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Creates a read future from a pinned input.
    ///
    /// # Parameters
    ///
    /// - `input`: Pinned asynchronous input.
    /// - `output`: Destination storage.
    ///
    /// # Returns
    ///
    /// Returns a future representing one read operation.
    #[inline(always)]
    pub const fn new(input: Pin<&'a mut I>, output: &'a mut [I::Item]) -> Self {
        Self {
            input,
            output,
            completed: false,
        }
    }
}

impl<I> Future for ReadFuture<'_, I>
where
    I: AsyncInput + ?Sized,
{
    /// Item count produced when the read becomes ready.
    type Output = Result<usize>;

    /// Polls the read operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready. A ready success
    /// contains the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the input.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "ReadFuture polled after completion");
        let result = this.input.as_mut().poll_read(cx, this.output);
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
