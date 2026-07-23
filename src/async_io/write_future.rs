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
use std::task::{
    Context,
    Poll,
};

use crate::AsyncOutput;

/// Future for one [`AsyncOutput`] write operation.
///
/// # Panics
///
/// [`Future::poll`] panics when called again after this future has returned
/// [`Poll::Ready`].
#[must_use = "futures do nothing unless polled"]
pub struct WriteFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    output: Pin<&'a mut O>,
    input: &'a [O::Item],
    completed: bool,
}

impl<'a, O> WriteFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Creates a write future from a pinned output.
    ///
    /// # Parameters
    ///
    /// * `output` - Pinned asynchronous output.
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future representing one write operation.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>, input: &'a [O::Item]) -> Self {
        Self {
            output,
            input,
            completed: false,
        }
    }
}

impl<O> Future for WriteFuture<'_, O>
where
    O: AsyncOutput + ?Sized,
{
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "WriteFuture polled after completion");
        let result = this.output.as_mut().poll_write(cx, this.input);
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
