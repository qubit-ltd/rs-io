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

use crate::AsyncInput;

/// Future for one [`AsyncInput`] read operation.
///
/// # Panics
///
/// [`Future::poll`] panics when called again after this future has returned
/// [`Poll::Ready`].
#[must_use = "futures do nothing unless polled"]
pub struct ReadFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    input: Pin<&'a mut I>,
    output: &'a mut [I::Item],
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
    /// * `input` - Pinned asynchronous input.
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future representing one read operation.
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
    type Output = Result<usize>;

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
