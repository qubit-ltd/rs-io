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
use crate::traits::normalize_async_error;
use crate::traits::validate_read_count;

/// Maximum successful inner operations performed by one outer poll.
pub(crate) const MAX_READY_OPERATIONS_PER_POLL: usize = 64;

/// Future that reads until its destination is full or EOF is reached.
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
/// use qubit_io::ReadFullyFuture;
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
/// let mut output = [0_u8; 2];
/// let mut future = ReadFullyFuture::new(Pin::new(&mut input), &mut output);
/// let mut context = Context::from_waker(Waker::noop());
/// assert_eq!(future.items_read(), 0);
/// assert!(matches!(
///     Future::poll(Pin::new(&mut future), &mut context),
///     Poll::Ready(Ok(1)),
/// ));
/// assert_eq!(future.items_read(), 1);
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
pub struct ReadFullyFuture<'a, I>
where
    I: AsyncInput + ?Sized,
{
    /// Input being read.
    input: Pin<&'a mut I>,
    /// Destination being filled.
    output: &'a mut [I::Item],
    /// Number of items read so far.
    read: usize,
    /// Whether the read operation has completed.
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
    /// - `input`: Pinned asynchronous input.
    /// - `output`: Destination storage.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves with the total number of items read.
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

impl<I> Future for ReadFullyFuture<'_, I>
where
    I: AsyncInput + ?Sized,
{
    /// Total item count produced when the operation becomes ready.
    type Output = Result<usize>;

    /// Polls the read-fully operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while more input may arrive. A ready success
    /// contains the number of items read before the destination filled or EOF
    /// was reached.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the input, normalizing `WouldBlock`
    /// and `Interrupted` to [`std::io::ErrorKind::InvalidData`] while retaining
    /// the original error in the cause chain. Returns
    /// [`std::io::ErrorKind::InvalidData`] if the input reports more items than
    /// the remaining destination can hold.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "ReadFullyFuture polled after completion");
        let mut ready_operations = 0_usize;
        while this.read < this.output.len() {
            let remaining = &mut this.output[this.read..];
            let requested = remaining.len();
            match this.input.as_mut().poll_read(cx, remaining) {
                Poll::Ready(Ok(0)) => {
                    this.completed = true;
                    return Poll::Ready(Ok(this.read));
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
        Poll::Ready(Ok(this.read))
    }
}
