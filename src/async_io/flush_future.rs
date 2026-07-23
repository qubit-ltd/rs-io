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

use crate::{
    AsyncOutput,
    traits::validate_async_error,
};

/// Future that flushes an [`AsyncOutput`].
#[must_use = "futures do nothing unless polled"]
pub struct FlushFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    output: Pin<&'a mut O>,
    completed: bool,
}

impl<'a, O> FlushFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Creates a flush future from a pinned output.
    ///
    /// # Parameters
    ///
    /// * `output` - Pinned asynchronous output.
    ///
    /// # Returns
    ///
    /// A future representing the flush operation.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>) -> Self {
        Self {
            output,
            completed: false,
        }
    }
}

impl<O> Future for FlushFuture<'_, O>
where
    O: AsyncOutput + ?Sized,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "FlushFuture polled after completion");
        let result = this
            .output
            .as_mut()
            .poll_flush(cx)
            .map(|result| result.map_err(validate_async_error));
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
