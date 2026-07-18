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

use crate::traits::validate_write_count;

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
    /// result containing a count in `0..=count`.
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
        let requested = input.len();
        // SAFETY: The full input slice is a valid source range.
        match unsafe { self.poll_write_unchecked(cx, input, 0, requested) } {
            Poll::Ready(Ok(written)) => Poll::Ready(
                validate_write_count(written, requested).map(|()| written),
            ),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
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
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<()>>;
}
