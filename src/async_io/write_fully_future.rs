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
#[must_use = "futures do nothing unless polled"]
pub struct WriteFullyFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    output: Pin<&'a mut O>,
    input: &'a [O::Item],
    written: usize,
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
    /// * `output` - Pinned asynchronous output.
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves after every item has been accepted.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>, input: &'a [O::Item]) -> Self {
        Self {
            output,
            input,
            written: 0,
            completed: false,
        }
    }
}

impl<O> Future for WriteFullyFuture<'_, O>
where
    O: AsyncOutput + ?Sized,
{
    type Output = Result<()>;

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
                Poll::Ready(Err(error))
                    if error.kind() == ErrorKind::Interrupted => {}
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
