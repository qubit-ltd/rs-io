// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncOutput;
use crate::CloseFuture;

/// Optional asynchronous capability for gracefully closing an output.
///
/// Closing is distinct from dropping the Rust value. Implementations complete
/// any required buffered output and underlying close operation asynchronously.
/// This capability remains separate from AsyncOutput because not every output
/// has a meaningful graceful-close operation.
///
/// # Examples
///
/// Close an output and verify that the operation completed:
///
/// ```
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// use qubit_io::AsyncClose;
/// use qubit_io::AsyncOutput;
///
/// struct Output { closed: bool }
///
/// impl AsyncOutput for Output {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>, _cx: &mut Context<'_>, _input: &[u8],
///         _index: usize, _count: usize,
///     ) -> Poll<Result<usize>> {
///         unreachable!("this example only closes the output")
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// impl AsyncClose for Output {
///     fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         self.closed = true;
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// let mut output = Output { closed: false };
/// {
///     let mut future = output.close_async();
///     let mut cx = Context::from_waker(Waker::noop());
///     assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(()))));
/// }
/// assert!(output.closed);
/// ```
pub trait AsyncClose: AsyncOutput {
    /// Polls the closing of this output.
    ///
    /// Before returning [`Poll::Pending`], the implementation must arrange for
    /// `cx`'s waker to be notified when closing may progress. `WouldBlock` and
    /// `Interrupted` must not cross this asynchronous boundary. A successful
    /// result means callers must no longer assume that writing remains valid.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when closing is pending.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] or the ready close result.
    ///
    /// # Errors
    ///
    /// Returns the close error reported by the implementation.
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

    /// Creates a future that closes this output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the close result.
    #[inline(always)]
    fn close_async(&mut self) -> CloseFuture<'_, Self>
    where
        Self: Sized + Unpin,
    {
        CloseFuture::new(Pin::new(self))
    }
}
