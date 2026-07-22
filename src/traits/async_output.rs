// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Result;
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use crate::traits::{
    validate_async_error,
    validate_write_count,
};
use crate::{
    FlushFuture,
    WriteFullyFuture,
    WriteFuture,
};

/// Minimal runtime-independent asynchronous output interface over items.
///
/// `AsyncOutput` expresses readiness through [`Poll`] and does not depend on a
/// particular executor. File publication operations such as commit and abort
/// intentionally do not belong to this byte-transfer abstraction.
pub trait AsyncOutput {
    /// The item type written to this output.
    type Item;

    /// Returns whether this output already buffers items internally.
    ///
    /// # Returns
    ///
    /// `true` when callers should avoid automatically adding another generic
    /// item buffer.
    #[inline(always)]
    #[must_use]
    fn is_buffered(&self) -> bool {
        false
    }

    /// Polls an indexed write without checking the source range.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when output is pending.
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] when no result is currently available, or a ready I/O
    /// result containing a count in `0..=count`. A zero `count` must
    /// immediately return `Poll::Ready(Ok(0))`.
    ///
    /// Before returning [`Poll::Pending`], the implementation must arrange for
    /// `cx`'s waker to be notified when progress may be possible. Neither
    /// `Poll::Pending` nor `Poll::Ready(Err(_))` may accept items.
    /// `WouldBlock` and `Interrupted` must not cross this asynchronous
    /// boundary; implementations must respectively register readiness or retry
    /// internally.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>>;

    /// Polls one write from the full source slice.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when output is pending.
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] or a ready result containing the accepted item count.
    ///
    /// # Errors
    ///
    /// Returns the implementation's output error. Returns
    /// [`std::io::ErrorKind::InvalidData`] if the implementation reports more
    /// items than requested.
    #[inline(always)]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
    ) -> Poll<Result<usize>> {
        if input.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let requested = input.len();
        // SAFETY: The full input slice is a valid source range.
        match unsafe { self.poll_write_unchecked(cx, input, 0, requested) } {
            Poll::Ready(Ok(written)) => Poll::Ready(
                validate_write_count(written, requested).map(|()| written),
            ),
            Poll::Ready(Err(error)) => {
                Poll::Ready(Err(validate_async_error(error)))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    /// Polls the flushing of internally buffered items.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when flushing is
    ///   pending.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] or the ready flush result.
    ///
    /// Before returning [`Poll::Pending`], the implementation must arrange for
    /// `cx`'s waker to be notified when flushing may progress. `WouldBlock` and
    /// `Interrupted` must not cross this asynchronous boundary.
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<()>>;

    /// Creates a future that performs one asynchronous write operation.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of accepted items.
    #[inline(always)]
    fn write_async<'a>(
        &'a mut self,
        input: &'a [Self::Item],
    ) -> WriteFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteFuture::new(Pin::new(self), input)
    }

    /// Creates a future that writes the entire source slice.
    ///
    /// The returned future reports [`std::io::ErrorKind::WriteZero`] when
    /// output makes no progress.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves when every item has been accepted.
    #[inline(always)]
    fn write_fully_async<'a>(
        &'a mut self,
        input: &'a [Self::Item],
    ) -> WriteFullyFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteFullyFuture::new(Pin::new(self), input)
    }

    /// Creates a future that flushes internally buffered items.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    #[inline(always)]
    fn flush_async(&mut self) -> FlushFuture<'_, Self>
    where
        Self: Sized + Unpin,
    {
        FlushFuture::new(Pin::new(self))
    }
}
