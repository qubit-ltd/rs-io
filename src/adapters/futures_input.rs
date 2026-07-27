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

use futures_io::AsyncRead;

use crate::{
    AsyncInput,
    UncheckedSlice,
    traits::validate_async_error,
};

/// Adapts a futures-io [`AsyncRead`] value to Qubit's [`AsyncInput`].
///
/// # Type Parameters
///
/// - `T`: Futures-io reader type.
#[must_use]
#[repr(transparent)]
pub struct FuturesInput<T> {
    /// Futures-io reader adapted as a Qubit input.
    inner: T,
}

impl<T> FuturesInput<T> {
    /// Creates an adapter around a futures-io reader.
    ///
    /// # Parameters
    ///
    /// - `inner`: Futures-io reader to adapt.
    ///
    /// # Returns
    ///
    /// Returns a Qubit input adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped futures-io reader.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    ///
    /// Returns the wrapped futures-io reader with mutable access.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped reader.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped reader without moving
    /// it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped reader.
    ///
    /// # Returns
    ///
    /// Returns the owned futures-io reader.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> AsyncInput for FuturesInput<T>
where
    T: AsyncRead,
{
    /// Byte item produced by a futures-io reader.
    type Item = u8;

    /// Polls an indexed read through the wrapped futures-io reader.
    ///
    /// A zero-length request completes immediately without polling `inner`.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Destination byte slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of bytes to read.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the reader is not ready. A ready result
    /// contains the number of bytes read.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped reader. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline]
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked futures-io read range exceeds output buffer"
        );
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        // SAFETY: The caller guarantees that the destination range is valid.
        let target =
            unsafe { UncheckedSlice::subslice_mut(output, index, count) };
        AsyncRead::poll_read(self.get_pin_mut(), cx, target)
            .map(|result| result.map_err(validate_async_error))
    }
}
