// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use super::read_fully_future::MAX_READY_OPERATIONS_PER_POLL;
use crate::AsyncOutput;
use crate::traits::normalize_async_error;
use crate::traits::validate_write_count;

/// Future that writes every item from its source slice.
///
/// Items accepted before cancellation remain written. The accepted count is
/// observable through [`Self::items_written`].
///
/// To preserve executor fairness, one outer poll performs a bounded number of
/// successful inner writes. The future self-wakes and returns [`Poll::Pending`]
/// when that budget is exhausted before completion.
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncOutput;
/// use qubit_io::WriteFullyFuture;
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// struct Output;
///
/// impl AsyncOutput for Output {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         _input: &[u8],
///         _index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         Poll::Ready(Ok(count))
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// let mut output = Output;
/// let input = [1, 2, 3];
/// let mut future = Box::pin(WriteFullyFuture::new(Pin::new(&mut output), &input));
/// let mut context = Context::from_waker(Waker::noop());
/// assert!(matches!(future.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
/// assert_eq!(future.items_written(), input.len());
/// ```
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
    /// completion. `WouldBlock` and `Interrupted` are normalized to
    /// [`ErrorKind::InvalidData`] with the original error in the cause chain.
    /// Returns [`ErrorKind::InvalidData`] if the output reports
    /// accepting more items than the remaining source contains. Other errors
    /// are propagated from the output.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "WriteFullyFuture polled after completion");
        let mut ready_operations = 0_usize;
        while this.written < this.input.len() {
            let remaining = &this.input[this.written..];
            let requested = remaining.len();
            match this.output.as_mut().poll_write(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    this.completed = true;
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole output range",
                    )));
                }
                Poll::Ready(Ok(written)) => {
                    if let Err(error) = validate_write_count(written, requested) {
                        this.completed = true;
                        return Poll::Ready(Err(error));
                    }
                    this.written += written;
                    ready_operations += 1;
                    if this.written < this.input.len() && ready_operations >= MAX_READY_OPERATIONS_PER_POLL {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Err(error)) => {
                    this.completed = true;
                    return Poll::Ready(Err(normalize_async_error(error)));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        this.completed = true;
        Poll::Ready(Ok(()))
    }
}
