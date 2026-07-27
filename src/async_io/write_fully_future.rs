// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::{
    Error,
    ErrorKind,
    Result,
};
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use crate::AsyncOutput;

/// Future that writes every item from its source slice.
///
/// Items accepted before cancellation remain written. The accepted count is
/// observable through [`Self::items_written`].
///
/// # Panics
///
/// [`Future::poll`] panics when called again after this future has returned
/// [`Poll::Ready`].
///
/// # Type Parameters
///
/// - `'a`: Shared lifetime of the output borrow and source slice.
/// - `O`: Asynchronous output type.
#[must_use = "futures do nothing unless polled"]
pub struct WriteFullyFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Output being written.
    output: Pin<&'a mut O>,
    /// Source whose items must all be written.
    input: &'a [O::Item],
    /// Number of items written so far.
    written: usize,
    /// Whether the write operation has completed.
    completed: bool,
}

impl<'a, O> WriteFullyFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Creates a write-fully future from a pinned output.
    ///
    /// # Parameters
    ///
    /// - `output`: Pinned asynchronous output.
    /// - `input`: Source storage.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves after every item has been accepted.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>, input: &'a [O::Item]) -> Self {
        Self {
            output,
            input,
            written: 0,
            completed: false,
        }
    }

    /// Returns the number of items written so far.
    ///
    /// # Returns
    ///
    /// Returns the progress retained across polls.
    #[inline(always)]
    #[must_use]
    pub const fn items_written(&self) -> usize {
        self.written
    }
}

impl<O> Future for WriteFullyFuture<'_, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Result produced when the write-fully operation becomes ready.
    type Output = Result<()>;

    /// Polls the write-fully operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while more output capacity is needed, or
    /// [`Poll::Ready`] after all items are written or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::WriteZero`] if the output accepts no item before
    /// completion. Other errors are propagated from the output.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "WriteFullyFuture polled after completion");
        while this.written < this.input.len() {
            let remaining = &this.input[this.written..];
            match this.output.as_mut().poll_write(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    this.completed = true;
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole output range",
                    )));
                }
                Poll::Ready(Ok(written)) => this.written += written,
                Poll::Ready(Err(error)) => {
                    this.completed = true;
                    return Poll::Ready(Err(error));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        this.completed = true;
        Poll::Ready(Ok(()))
    }
}
