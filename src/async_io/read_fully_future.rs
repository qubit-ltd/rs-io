// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::{
    ErrorKind,
    Result,
};
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use crate::AsyncInput;

/// Future that reads until its destination is full or EOF is reached.
#[must_use = "futures do nothing unless polled"]
pub struct ReadFullyFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    input: Pin<&'a mut I>,
    output: &'a mut [I::Item],
    read: usize,
    completed: bool,
}

impl<'a, I> ReadFullyFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Creates a read-fully future from a pinned input.
    ///
    /// # Parameters
    ///
    /// * `input` - Pinned asynchronous input.
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the total number of items read.
    #[inline(always)]
    pub const fn new(input: Pin<&'a mut I>, output: &'a mut [I::Item]) -> Self {
        Self {
            input,
            output,
            read: 0,
            completed: false,
        }
    }
}

impl<I> Future for ReadFullyFuture<'_, I>
where
    I: AsyncInput + ?Sized,
{
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "ReadFullyFuture polled after completion");
        while this.read < this.output.len() {
            let remaining = &mut this.output[this.read..];
            match this.input.as_mut().poll_read(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    this.completed = true;
                    return Poll::Ready(Ok(this.read));
                }
                Poll::Ready(Ok(read)) => this.read += read,
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
        Poll::Ready(Ok(this.read))
    }
}
