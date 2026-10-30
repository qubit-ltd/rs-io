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
use std::task::Context;
use std::task::Poll;

use crate::AsyncInput;
use crate::traits::validate_read_count;

/// Future for one [`AsyncInput`] read operation.
///
/// # Examples
///
/// Poll one in-memory read and inspect both its count and data:
///
/// ```
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::{Context, Poll, Waker};
///
/// use qubit_io::{AsyncInput, ReadFuture};
///
/// struct Input { read: bool }
///
/// impl AsyncInput for Input {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8],
///         index: usize, count: usize,
///     ) -> Poll<Result<usize>> {
///         if count == 0 || self.read { return Poll::Ready(Ok(0)); }
///         output[index] = b'x';
///         self.read = true;
///         Poll::Ready(Ok(1))
///     }
/// }
///
/// let mut input = Input { read: false };
/// let mut output = [0_u8; 2];
/// let mut future = ReadFuture::new(Pin::new(&mut input), &mut output);
/// let mut cx = Context::from_waker(Waker::noop());
/// assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(1))));
/// drop(future);
/// assert_eq!(output, [b'x', 0]);
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
pub struct ReadFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Input being read.
    input: Pin<&'a mut I>,
    /// Destination for the read.
    output: &'a mut [I::Item],
    /// Whether the read operation has completed.
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
    /// - `input`: Pinned asynchronous input.
    /// - `output`: Destination storage.
    ///
    /// # Returns
    ///
    /// Returns a future representing one read operation.
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
    /// Item count produced when the read becomes ready.
    type Output = Result<usize>;

    /// Polls the read operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready. A ready success
    /// contains the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the input. Returns
    /// [`std::io::ErrorKind::InvalidData`] if the input reports more items than
    /// requested.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "ReadFuture polled after completion");
        let requested = this.output.len();
        let result = this.input.as_mut().poll_read(cx, this.output);
        let result = result.map(|result| {
            result.and_then(|read| {
                validate_read_count(read, requested)?;
                Ok(read)
            })
        });
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
