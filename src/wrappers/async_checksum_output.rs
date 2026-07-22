// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    hash::Hasher,
    io,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use crate::{
    AsyncClose,
    AsyncOutput,
};

/// Asynchronous byte output that hashes successfully accepted bytes.
///
/// Pending and failed writes do not change the hasher. The checksum algorithm
/// and stability guarantees are those of the supplied [`Hasher`].
#[derive(Debug)]
pub struct AsyncChecksumOutput<O, H> {
    inner: O,
    hasher: H,
}

impl<O, H> AsyncClose for AsyncChecksumOutput<O, H>
where
    O: AsyncClose<Item = u8>,
    H: Hasher,
{
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }.poll_close(cx)
    }
}

impl<O, H> AsyncChecksumOutput<O, H>
where
    H: Hasher,
{
    /// Creates a checksum-tracking asynchronous output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous byte output to wrap.
    /// - `hasher`: Hasher updated after successful writes.
    ///
    /// # Returns
    ///
    /// Returns an output with the supplied initial hasher state.
    #[must_use]
    pub const fn new(inner: O, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    ///
    /// # Returns
    ///
    /// Returns [`Hasher::finish`] for the current state.
    #[must_use]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte output.
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte output.
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Returns a shared reference to the hasher.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state.
    #[must_use]
    pub const fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns a mutable reference to the hasher.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state mutably.
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns both owned parts.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and final hasher state.
    #[must_use]
    pub fn into_parts(self) -> (O, H) {
        (self.inner, self.hasher)
    }
}

impl<O, H> AsyncOutput for AsyncChecksumOutput<O, H>
where
    O: AsyncOutput<Item = u8>,
    H: Hasher,
{
    type Item = u8;

    /// Preserves the wrapped output's buffering declaration.
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write and hashes only bytes in a successful ready result.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
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
                this.hasher.write(&input[index..index + written]);
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
