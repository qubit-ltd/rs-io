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
    task::{Context, Poll},
};

use crate::{AsyncClose, AsyncOutput, traits::normalize_async_error};

/// Asynchronous output that counts successfully accepted items.
///
/// # Type Parameters
///
/// - `O`: Wrapped asynchronous output type.
#[must_use]
#[derive(Debug)]
pub struct AsyncCountingOutput<O> {
    /// Output whose successful writes are counted.
    inner: O,
    /// Saturating count of accepted items.
    items_written: u64,
}

impl<O> AsyncClose for AsyncCountingOutput<O>
where
    O: AsyncClose,
{
    /// Polls closing through the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while closing is incomplete, otherwise a
    /// ready success result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output. Invalid asynchronous
    /// error kinds are normalized to [`io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
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
    #[inline(always)]
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
    #[inline(always)]
    #[must_use]
    pub const fn items_written(&self) -> u64 {
        self.items_written
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// Writes performed directly on the returned output are not included in
    /// this wrapper's item count.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous output.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous output.
    #[inline(always)]
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
    #[inline(always)]
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.items_written
    }
}

impl<O> AsyncOutput for AsyncCountingOutput<O>
where
    O: AsyncOutput,
{
    /// Item type counted after successful writes.
    type Item = O::Item;

    /// Preserves the wrapped output's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write and counts only a successful ready result.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the output is not ready. A ready success
    /// contains the number of items accepted and added to the saturating count.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped output without changing the
    /// count.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
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
                this.items_written = this.items_written.saturating_add(written_u64);
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Polls the wrapped output's flush operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while flushing is incomplete, otherwise a
    /// ready success result.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output. Invalid asynchronous
    /// error kinds are normalized to [`io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
