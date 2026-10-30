// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use digest::Digest;

use crate::AsyncByteInput;
use crate::DEFAULT_COPY_BUFFER_SIZE;
use crate::async_io::MAX_READY_OPERATIONS_PER_POLL;
use crate::traits::normalize_async_error;
use crate::traits::validate_read_count;

/// Future that reads the remaining bytes of an input and returns their digest.
///
/// # Examples
///
/// Read a short in-memory input to EOF and verify its SHA-256 digest:
///
/// ```
/// use std::future::Future;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::{Context, Poll, Waker};
///
/// use qubit_io::{AsyncInput, DigestAllFuture};
/// use sha2::{Digest, Sha256};
///
/// struct Input { bytes: &'static [u8], position: usize }
///
/// impl AsyncInput for Input {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8],
///         index: usize, count: usize,
///     ) -> Poll<Result<usize>> {
///         let read = (self.bytes.len() - self.position).min(count);
///         output[index..index + read]
///             .copy_from_slice(&self.bytes[self.position..self.position + read]);
///         self.position += read;
///         Poll::Ready(Ok(read))
///     }
/// }
///
/// let mut input = Input { bytes: b"abc", position: 0 };
/// let mut future = DigestAllFuture::<_, Sha256>::new(Pin::new(&mut input));
/// let mut cx = Context::from_waker(Waker::noop());
/// let digest = match Future::poll(Pin::new(&mut future), &mut cx) {
///     Poll::Ready(Ok(digest)) => digest,
///     other => panic!("unexpected result: {other:?}"),
/// };
/// assert_eq!(digest, Sha256::digest(b"abc"));
/// ```
///
/// Available with the `digest` feature. Uses a fresh algorithm state and a
/// fixed-size bulk-read buffer. Progress survives `Pending`; cancellation
/// discards the partial digest without rewinding the input. One poll performs
/// at most 64 successful reads before self-waking to preserve executor
/// fairness.
///
/// # Panics
///
/// Polling after completion panics, including after an error.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input.
/// - `I`: Asynchronous byte input, including pinned `!Unpin` inputs and trait
///   objects.
/// - `D`: Fixed-output digest algorithm.
#[must_use = "futures do nothing unless polled"]
pub struct DigestAllFuture<'a, I, D>
where
    I: AsyncByteInput + ?Sized,
    D: Digest,
{
    /// Input kept pinned for the entire borrow.
    input: Pin<&'a mut I>,
    /// Algorithm state, removed when the operation completes.
    hasher: Option<D>,
    /// Reused bulk-read storage, independent of input length.
    buffer: [u8; DEFAULT_COPY_BUFFER_SIZE],
}

// No field is structurally pinned: the input itself stays behind Pin<&mut I>,
// and the algorithm is never exposed through a pinned reference.
impl<I, D> Unpin for DigestAllFuture<'_, I, D>
where
    I: AsyncByteInput + ?Sized,
    D: Digest,
{
}

impl<'a, I, D> DigestAllFuture<'a, I, D>
where
    I: AsyncByteInput + ?Sized,
    D: Digest,
{
    /// Creates a future with a fresh digest state at the current input
    /// position.
    ///
    /// # Parameters
    ///
    /// - `input`: Pinned byte input to read until EOF.
    ///
    /// # Returns
    ///
    /// Returns an unpolled future; construction performs no I/O.
    #[inline]
    pub fn new(input: Pin<&'a mut I>) -> Self {
        Self {
            input,
            hasher: Some(D::new()),
            buffer: [0; DEFAULT_COPY_BUFFER_SIZE],
        }
    }
}

impl<I, D> Future for DigestAllFuture<'_, I, D>
where
    I: AsyncByteInput + ?Sized,
    D: Digest,
{
    /// Full digest of bytes read from the initial position to EOF.
    type Output = io::Result<digest::Output<D>>;

    /// Polls bulk reads, retaining digest state across pending operations.
    ///
    /// # Errors
    ///
    /// Returns the input's read error, normalizing `WouldBlock` and
    /// `Interrupted` to [`io::ErrorKind::InvalidData`] while retaining the
    /// original error in the cause chain, or [`io::ErrorKind::InvalidData`] if
    /// the input reports more bytes than requested. Errors do not rewind
    /// consumed bytes or return a partial digest.
    ///
    /// # Panics
    ///
    /// Panics when polled after returning `Ready`.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(this.hasher.is_some(), "DigestAllFuture polled after completion");
        for _ in 0..MAX_READY_OPERATIONS_PER_POLL {
            match this.input.as_mut().poll_read(cx, &mut this.buffer) {
                Poll::Ready(Ok(0)) => {
                    let hasher = this.hasher.take().expect("digest state is present until completion");
                    return Poll::Ready(Ok(hasher.finalize()));
                }
                Poll::Ready(Ok(read)) => {
                    if let Err(error) = validate_read_count(read, this.buffer.len()) {
                        this.hasher = None;
                        return Poll::Ready(Err(error));
                    }
                    this.hasher
                        .as_mut()
                        .expect("digest state is present until completion")
                        .update(&this.buffer[..read]);
                }
                Poll::Ready(Err(error)) => {
                    this.hasher = None;
                    return Poll::Ready(Err(normalize_async_error(error)));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
