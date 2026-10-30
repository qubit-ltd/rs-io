// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
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
use crate::AsyncInput;
use crate::traits::normalize_async_error;
use crate::traits::validate_read_count;

/// Future that reads until its destination is full.
///
/// Progress is retained in the destination and is observable through
/// [`Self::items_read`] if the future is cancelled.
///
/// To preserve executor fairness, one outer poll performs a bounded number of
/// successful inner reads. The future self-wakes and returns [`Poll::Pending`]
/// when that budget is exhausted before completion.
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncInput;
/// use qubit_io::ReadExactFuture;
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// struct OneByteInput(bool);
///
/// impl AsyncInput for OneByteInput {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         mut self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         output: &mut [u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         if count == 0 || self.0 {
///             return Poll::Ready(Ok(0));
///         }
///         output[index] = b'x';
///         self.0 = true;
///         Poll::Ready(Ok(1))
///     }
/// }
///
/// let mut input = OneByteInput(false);
/// let mut output = [0_u8; 1];
/// let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
/// let mut context = Context::from_waker(Waker::noop());
/// assert_eq!(future.items_read(), 0);
/// assert!(matches!(
///     Future::poll(Pin::new(&mut future), &mut context),
///     Poll::Ready(Ok(())),
/// ));
/// assert_eq!(future.items_read(), 1);
/// drop(future);
/// assert_eq!(output, [b'x']);
/// ```
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
pub struct ReadExactFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Pinned mutable borrow of the input used for every read poll.
    ///
    /// The pin preserves the input's address while this future is polled, and
    /// the borrow lasts for the future's lifetime.
    input: Pin<&'a mut I>,
    /// Mutable destination slice that receives each item read.
    ///
    /// The slice remains exclusively borrowed for the future's lifetime, so
    /// its initialized prefix preserves progress if the future is cancelled.
    output: &'a mut [I::Item],
    /// Number of items successfully read into the destination.
    read: usize,
    /// Whether the operation has returned a ready result.
    completed: bool,
}

impl<'a, I> ReadExactFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Creates an exact-read future from a pinned input.
    ///
    /// # Parameters
    ///
    /// - `input`: Pinned asynchronous input.
    /// - `output`: Destination slice that must be filled.
    ///
    /// # Returns
    ///
    /// Returns a future that completes after filling `output`.
    #[inline(always)]
    pub const fn new(input: Pin<&'a mut I>, output: &'a mut [I::Item]) -> Self {
        Self {
            input,
            output,
            read: 0,
            completed: false,
        }
    }

    /// Returns the number of items read so far.
    ///
    /// # Returns
    ///
    /// Returns the progress retained across polls.
    #[inline(always)]
    #[must_use]
    pub const fn items_read(&self) -> usize {
        self.read
    }
}

impl<I> Future for ReadExactFuture<'_, I>
where
    I: AsyncInput + ?Sized,
{
    /// Result produced when the exact-read operation becomes ready.
    type Output = Result<()>;

    /// Polls the exact-read operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while more input is needed, or
    /// [`Poll::Ready`] after the destination is filled or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] when EOF is reached before the
    /// destination is full. `WouldBlock` and `Interrupted` are normalized to
    /// [`ErrorKind::InvalidData`] with the original error in the cause chain.
    /// Returns [`ErrorKind::InvalidData`] if the input
    /// reports more items than the remaining destination can hold. Other
    /// errors are propagated from the input.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "ReadExactFuture polled after completion");
        let mut ready_operations = 0_usize;
        while this.read < this.output.len() {
            let remaining = &mut this.output[this.read..];
            let requested = remaining.len();
            match this.input.as_mut().poll_read(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    this.completed = true;
                    return Poll::Ready(Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "failed to fill whole input range",
                    )));
                }
                Poll::Ready(Ok(read)) => {
                    if let Err(error) = validate_read_count(read, requested) {
                        this.completed = true;
                        return Poll::Ready(Err(error));
                    }
                    this.read += read;
                    ready_operations += 1;
                    if this.read < this.output.len() && ready_operations >= MAX_READY_OPERATIONS_PER_POLL {
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
