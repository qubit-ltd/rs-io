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
#[repr(transparent)]
pub struct FuturesInput<T> {
    inner: T,
}

impl<T> FuturesInput<T> {
    /// Creates an adapter around a futures-io reader.
    #[inline(always)]
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped reader.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped reader.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped reader.
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
    type Item = u8;

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
