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

use crate::AsyncInput;

/// Asynchronous byte input that hashes successfully returned bytes.
///
/// Pending and failed reads do not change the hasher. The checksum algorithm
/// and stability guarantees are those of the supplied [`Hasher`].
#[derive(Debug)]
pub struct AsyncChecksumInput<I, H> {
    inner: I,
    hasher: H,
}

impl<I, H> AsyncChecksumInput<I, H>
where
    H: Hasher,
{
    /// Creates a checksum-tracking asynchronous input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous byte input to wrap.
    /// - `hasher`: Hasher updated after successful reads.
    ///
    /// # Returns
    ///
    /// Returns an input with the supplied initial hasher state.
    #[must_use]
    pub const fn new(inner: I, hasher: H) -> Self {
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

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte input.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte input.
    pub fn inner_mut(&mut self) -> &mut I {
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
    /// Returns the wrapped input and final hasher state.
    #[must_use]
    pub fn into_parts(self) -> (I, H) {
        (self.inner, self.hasher)
    }
}

impl<I, H> AsyncInput for AsyncChecksumInput<I, H>
where
    I: AsyncInput<Item = u8>,
    H: Hasher,
{
    type Item = u8;

    /// Preserves the wrapped input's buffering declaration.
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a read and hashes only bytes in a successful ready result.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let destination = &mut output[index..index + count];
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_read(cx, destination) {
            Poll::Ready(Ok(read)) => {
                this.hasher.write(&output[index..index + read]);
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
