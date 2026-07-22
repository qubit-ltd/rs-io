// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
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

use crate::AsyncClose;

/// Future that closes an [`AsyncClose`] output.
#[must_use = "futures do nothing unless polled"]
pub struct CloseFuture<'a, O>
where
    O: AsyncClose + ?Sized,
{
    /// Output being closed.
    output: Pin<&'a mut O>,
    /// Whether the close operation has completed.
    completed: bool,
}

impl<'a, O> CloseFuture<'a, O>
where
    O: AsyncClose + ?Sized,
{
    /// Creates a close future from a pinned output.
    ///
    /// # Parameters
    ///
    /// * `output` - Pinned asynchronous output.
    ///
    /// # Returns
    ///
    /// A future representing the close operation.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>) -> Self {
        Self {
            output,
            completed: false,
        }
    }
}

impl<O> Future for CloseFuture<'_, O>
where
    O: AsyncClose + ?Sized,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "CloseFuture polled after completion");
        let result = this.output.as_mut().poll_close(cx);
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
