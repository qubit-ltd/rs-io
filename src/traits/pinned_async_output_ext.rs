// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::AsyncClose;
use crate::AsyncOutput;
use crate::CloseFuture;
use crate::FlushFuture;
use crate::WriteFullyFuture;
use crate::WriteFuture;

/// Convenience futures for an already pinned asynchronous output.
///
/// This extension serves outputs that are not Unpin and pinned output trait
/// objects, for which AsyncOutput convenience methods taking a movable mutable
/// reference are unavailable.
///
/// # Examples
///
/// Create a write future from an already pinned output:
///
/// ```
/// use std::io::Result;
/// use std::future::Future;
/// use std::pin::Pin;
/// use std::task::{Context, Poll, Waker};
///
/// use qubit_io::AsyncOutput;
/// use qubit_io::PinnedAsyncOutputExt;
///
/// struct VecOutput(Vec<u8>);
///
/// impl AsyncOutput for VecOutput {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         input: &[u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         self.get_mut().0.extend_from_slice(&input[index..index + count]);
///         Poll::Ready(Ok(count))
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// let mut output = VecOutput(Vec::new());
/// let mut pinned = Pin::new(&mut output);
/// {
///     let mut future = pinned.write_async(&[1, 2, 3]);
///     let mut cx = Context::from_waker(Waker::noop());
///     assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(3))));
/// }
/// drop(pinned);
/// assert_eq!(output.0, [1, 2, 3]);
/// ```
pub trait PinnedAsyncOutputExt {
    /// The pinned asynchronous output type.
    type Output: AsyncOutput + ?Sized;

    /// Creates a future that performs one write.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of accepted items.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if a write fails, or
    /// [`std::io::ErrorKind::InvalidData`] if the output reports accepting more
    /// items than requested.
    /// The returned future converts `WouldBlock` and `Interrupted` errors to
    /// `InvalidData`, retaining the original error in the cause chain. It does
    /// not retry those errors or undo progress from earlier successful calls.
    fn write_async<'a>(&'a mut self, input: &'a [<Self::Output as AsyncOutput>::Item])
    -> WriteFuture<'a, Self::Output>;

    /// Creates a future that writes the entire source.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves after every item has been accepted.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if a write fails, with
    /// [`std::io::ErrorKind::WriteZero`] if the output makes no progress, or
    /// with [`std::io::ErrorKind::InvalidData`] if it reports an impossible
    /// item count.
    /// The returned future converts `WouldBlock` and `Interrupted` errors to
    /// `InvalidData`, retaining the original error in the cause chain. It does
    /// not retry those errors or undo progress from earlier successful calls.
    fn write_fully_async<'a>(
        &'a mut self,
        input: &'a [<Self::Output as AsyncOutput>::Item],
    ) -> WriteFullyFuture<'a, Self::Output>;

    /// Creates a future that flushes the output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if flushing fails, or with
    /// [`std::io::ErrorKind::InvalidData`] if the error kind is normalized at
    /// the asynchronous boundary.
    fn flush_async(&mut self) -> FlushFuture<'_, Self::Output>;

    /// Creates a future that closes the output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the close result.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if closing fails, or with
    /// [`std::io::ErrorKind::InvalidData`] if the error kind is normalized at
    /// the asynchronous boundary.
    fn close_async(&mut self) -> CloseFuture<'_, Self::Output>
    where
        Self::Output: AsyncClose;
}

impl<O> PinnedAsyncOutputExt for Pin<&mut O>
where
    O: AsyncOutput + ?Sized,
{
    /// The asynchronous output behind this pinned reference.
    type Output = O;

    /// Creates a single-write future for this pinned output.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of accepted items.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if a write fails, or
    /// [`std::io::ErrorKind::InvalidData`] if the output reports accepting more
    /// items than requested.
    /// The returned future converts `WouldBlock` and `Interrupted` errors to
    /// `InvalidData`, retaining the original error in the cause chain. It does
    /// not retry those errors or undo progress from earlier successful calls.
    #[inline(always)]
    fn write_async<'a>(&'a mut self, input: &'a [O::Item]) -> WriteFuture<'a, Self::Output> {
        WriteFuture::new(self.as_mut(), input)
    }

    /// Creates a write-fully future for this pinned output.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves after every item has been accepted.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if a write fails, with
    /// [`std::io::ErrorKind::WriteZero`] if the output makes no progress, or
    /// with [`std::io::ErrorKind::InvalidData`] if it reports an impossible
    /// item count.
    /// The returned future converts `WouldBlock` and `Interrupted` errors to
    /// `InvalidData`, retaining the original error in the cause chain. It does
    /// not retry those errors or undo progress from earlier successful calls.
    #[inline(always)]
    fn write_fully_async<'a>(&'a mut self, input: &'a [O::Item]) -> WriteFullyFuture<'a, Self::Output> {
        WriteFullyFuture::new(self.as_mut(), input)
    }

    /// Creates a flush future for this pinned output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if flushing fails, or with
    /// [`std::io::ErrorKind::InvalidData`] if the error kind is normalized at
    /// the asynchronous boundary.
    #[inline(always)]
    fn flush_async(&mut self) -> FlushFuture<'_, Self::Output> {
        FlushFuture::new(self.as_mut())
    }

    /// Creates a close future for this pinned output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the close result.
    ///
    /// # Errors
    ///
    /// Resolves with the output error if closing fails, or with
    /// [`std::io::ErrorKind::InvalidData`] if the error kind is normalized at
    /// the asynchronous boundary.
    #[inline(always)]
    fn close_async(&mut self) -> CloseFuture<'_, Self::Output>
    where
        Self::Output: AsyncClose,
    {
        CloseFuture::new(self.as_mut())
    }
}
