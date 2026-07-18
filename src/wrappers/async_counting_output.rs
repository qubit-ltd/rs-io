// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    io,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use crate::AsyncOutput;

/// Asynchronous output that counts successfully accepted items.
#[derive(Debug)]
pub struct AsyncCountingOutput<O> {
    inner: O,
    items_written: u64,
}

impl<O> AsyncCountingOutput<O> {
    /// Creates a counting asynchronous output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous output to wrap.
    ///
    /// # Returns
    ///
    /// Returns an output whose item count starts at zero.
    #[must_use]
    pub const fn new(inner: O) -> Self {
        Self {
            inner,
            items_written: 0,
        }
    }

    /// Returns the successfully accepted item count.
    ///
    /// # Returns
    ///
    /// Returns a count that saturates at [`u64::MAX`].
    #[must_use]
    pub const fn items_written(&self) -> u64 {
        self.items_written
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous output.
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> AsyncCountingOutput<O>
where
    O: AsyncOutput<Item = u8>,
{
    /// Returns the successfully accepted byte count.
    ///
    /// # Returns
    ///
    /// Returns the same value as [`Self::items_written`].
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.items_written
    }
}

impl<O> AsyncOutput for AsyncCountingOutput<O>
where
    O: AsyncOutput,
{
    type Item = O::Item;

    /// Preserves the wrapped output's buffering declaration.
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write and counts only a successful ready result.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let source = &input[index..index + count];
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_write(cx, source) {
            Poll::Ready(Ok(written)) => {
                let written_u64 = u64::try_from(written).unwrap_or(u64::MAX);
                this.items_written =
                    this.items_written.saturating_add(written_u64);
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Polls the wrapped output's flush operation.
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }.poll_flush(cx)
    }
}
