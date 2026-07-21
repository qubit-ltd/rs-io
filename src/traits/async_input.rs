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

use crate::{
    ReadFullyFuture,
    ReadFuture,
};
use crate::traits::validate_read_count;

/// Minimal runtime-independent asynchronous input interface over items.
///
/// `AsyncInput` expresses readiness through [`Poll`] and does not depend on a
/// particular executor. Implementations may be pinned and therefore are never
/// moved by the polling methods.
pub trait AsyncInput {
    /// The item type read from this input.
    type Item;

    /// Returns whether this input already buffers items internally.
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

    /// Polls an indexed read without checking the destination range.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when input is pending.
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] when no result is currently available, or a ready I/O
    /// result containing a count in `0..=count`. A ready zero count denotes
    /// end of input when `count` is nonzero.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>>;

    /// Polls a read into the full destination slice.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when input is pending.
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] or a ready result containing the number of items read.
    ///
    /// # Errors
    ///
    /// Returns the implementation's input error. Returns
    /// [`std::io::ErrorKind::InvalidData`] if the implementation reports more
    /// items than requested.
    #[inline(always)]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
    ) -> Poll<Result<usize>> {
        let requested = output.len();
        // SAFETY: The full output slice is a valid destination range.
        match unsafe { self.poll_read_unchecked(cx, output, 0, requested) } {
            Poll::Ready(Ok(read)) => {
                Poll::Ready(validate_read_count(read, requested).map(|()| read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Creates a future that performs one asynchronous read operation.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of items read.
    #[inline(always)]
    fn read_async<'a>(
        &'a mut self,
        output: &'a mut [Self::Item],
    ) -> ReadFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFuture::new(Pin::new(self), output)
    }

    /// Creates a future that fills a destination as far as possible.
    ///
    /// The returned future retries interrupted operations and stops when the
    /// destination is full or the input reports EOF.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the total number of items read.
    #[inline(always)]
    fn read_fully_async<'a>(
        &'a mut self,
        output: &'a mut [Self::Item],
    ) -> ReadFullyFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFullyFuture::new(Pin::new(self), output)
    }
}
