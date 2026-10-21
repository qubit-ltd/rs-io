// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
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

use crate::AsyncClose;
use crate::traits::normalize_async_error;

/// Future that closes an [`AsyncClose`] output.
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
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncClose;
/// use qubit_io::AsyncOutput;
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
///         _input: &[u8],
///         _index: usize,
///         _count: usize,
///     ) -> Poll<Result<usize>> {
///         unreachable!("the example only closes the output")
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
/// impl AsyncClose for Output {
///     fn poll_close(
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
/// let mut output = Output;
/// let waker = Waker::from(Arc::new(NoopWake));
/// let mut context = Context::from_waker(&waker);
/// let mut close = Box::pin(output.close_async());
/// assert!(matches!(close.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
/// ```
#[must_use = "futures do nothing unless polled"]
pub struct CloseFuture<'a, O>
where
    O: AsyncClose + ?Sized,
{
    /// Output being closed.
    output: Pin<&'a mut O>,
    /// Whether the close operation has completed.
    completed: bool,
}

impl<'a, O> CloseFuture<'a, O>
where
    O: AsyncClose + ?Sized,
{
    /// Creates a close future from a pinned output.
    ///
    /// # Parameters
    ///
    /// - `output`: Pinned asynchronous output.
    ///
    /// # Returns
    ///
    /// Returns a future representing the close operation.
    #[inline(always)]
    pub const fn new(output: Pin<&'a mut O>) -> Self {
        Self {
            output,
            completed: false,
        }
    }
}

impl<O> Future for CloseFuture<'_, O>
where
    O: AsyncClose + ?Sized,
{
    /// Result produced when the close operation becomes ready.
    type Output = Result<()>;

    /// Polls the close operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while closing is incomplete. A ready result
    /// indicates whether closing succeeded.
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
        assert!(!this.completed, "CloseFuture polled after completion");
        let result = this
            .output
            .as_mut()
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error));
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}
