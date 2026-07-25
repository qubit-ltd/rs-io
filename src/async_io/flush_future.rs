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
///
/// # Panics
///
/// [`Future::poll`] panics when called again after this future has returned
/// [`Poll::Ready`].
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed output.
/// - `O`: Asynchronous output type.
#[must_use = "futures do nothing unless polled"]
pub struct FlushFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Output being flushed.
    output: Pin<&'a mut O>,
    /// Whether the flush operation has completed.
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
    /// - `output`: Pinned asynchronous output.
    ///
    /// # Returns
    ///
    /// Returns a future representing the flush operation.
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
    /// Result produced when the flush operation becomes ready.
    type Output = Result<()>;

    /// Polls the flush operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while flushing is incomplete. A ready result
    /// indicates whether flushing succeeded.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the output. Invalid asynchronous error
    /// kinds are normalized to [`std::io::ErrorKind::InvalidData`].
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    #[inline]
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
