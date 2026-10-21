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

use crate::AsyncOutput;

/// Future for one [`AsyncOutput`] write operation.
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
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncOutput;
/// use qubit_io::WriteFuture;
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::sync::Arc;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Wake;
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
///         input: &[u8],
///         _index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         Poll::Ready(Ok(input.len().min(count)))
///     }
///
///     fn poll_flush(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///     ) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// struct NoopWake;
///
/// impl Wake for NoopWake {
///     fn wake(self: Arc<Self>) {}
/// }
///
/// let input = [1, 2, 3];
/// let mut output = Output;
/// let mut write = Box::pin(WriteFuture::new(Pin::new(&mut output), &input));
/// let waker = Waker::from(Arc::new(NoopWake));
/// let mut context = Context::from_waker(&waker);
/// assert!(matches!(write.as_mut().poll(&mut context), Poll::Ready(Ok(3))));
/// ```
#[must_use = "futures do nothing unless polled"]
pub struct WriteFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Output being written.
    output: Pin<&'a mut O>,
    /// Source for the write.
    input: &'a [O::Item],
    /// Whether the write operation has completed.
    completed: bool,
}

impl<'a, O> WriteFuture<'a, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Creates a write future from a pinned output.
    ///
    /// # Parameters
    ///
    /// - `output`: Pinned asynchronous output.
    /// - `input`: Source storage.
    ///
    /// # Returns
    ///
    /// Returns a future representing one write operation.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>, input: &'a [O::Item]) -> Self {
        Self {
            output,
            input,
            completed: false,
        }
    }
}

impl<O> Future for WriteFuture<'_, O>
where
    O: AsyncOutput + ?Sized,
{
    /// Item count produced when the write becomes ready.
    type Output = Result<usize>;

    /// Polls the write operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the output is not ready. A ready success
    /// contains the number of items accepted.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the output.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning [`Poll::Ready`].
    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(!this.completed, "WriteFuture polled after completion");
        let result = this.output.as_mut().poll_write(cx, this.input);
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
