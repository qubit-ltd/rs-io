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
