// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::{
    Context,
    Poll,
};

use futures_io::AsyncWrite;

use crate::util::UncheckedSlice;
use crate::{
    AsyncClose,
    AsyncOutput,
    traits::normalize_async_error,
};

/// Adapts a futures-io [`AsyncWrite`] value to Qubit's [`AsyncOutput`].
///
/// # Type Parameters
///
/// - `T`: Futures-io writer type.
#[must_use]
#[repr(transparent)]
pub struct FuturesOutput<T> {
    /// Futures-io writer adapted as a Qubit output.
    inner: T,
}

impl<T> FuturesOutput<T> {
    /// Creates an adapter around a futures-io writer.
    ///
    /// # Parameters
    ///
    /// - `inner`: Futures-io writer to adapt.
    ///
    /// # Returns
    ///
    /// Returns a Qubit output adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped futures-io writer.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped futures-io writer with mutable access.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped writer.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped writer without moving
    /// it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped writer.
    ///
    /// # Returns
    ///
    /// Returns the owned futures-io writer.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> AsyncOutput for FuturesOutput<T>
where
    T: AsyncWrite,
{
    /// Byte item accepted by a futures-io writer.
    type Item = u8;

    /// Polls an indexed write through the wrapped futures-io writer.
    ///
    /// A zero-length request completes immediately without polling `inner`.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source byte slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of bytes to write.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the writer is not ready. A ready result
    /// contains the number of bytes accepted.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped writer. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested input range does not fit.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline]
    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        // SAFETY: The caller guarantees that the source range is valid.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        AsyncWrite::poll_write(self.get_pin_mut(), cx, source)
            .map(|result| result.map_err(normalize_async_error))
    }

    /// Polls flushing through the wrapped futures-io writer.
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
    /// Returns an I/O error reported by the wrapped writer. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_flush(self.get_pin_mut(), cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}

impl<T> AsyncClose for FuturesOutput<T>
where
    T: AsyncWrite,
{
    /// Polls closing through the wrapped futures-io writer.
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
    /// Returns an I/O error reported by the wrapped writer. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        AsyncWrite::poll_close(self.get_pin_mut(), cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
