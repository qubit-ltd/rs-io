// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use digest::Digest;

use crate::AsyncByteOutput;
use crate::AsyncClose;
use crate::AsyncOutput;
use crate::traits::normalize_async_error;
use crate::traits::validate_write_count;

/// Asynchronous byte output that computes a digest of successfully accepted
/// bytes.
///
/// Available with the `digest` feature.
///
/// Pending and failed writes do not change the hasher. The algorithm is
/// supplied by the caller. The digest covers transferred bytes in order,
/// independently of transfer chunk boundaries.
///
/// # Examples
///
/// ```
/// use std::future::Future;
/// use sha2::Digest;
/// use sha2::Sha256;
/// use std::io;
/// use std::pin::Pin;
/// use std::sync::Arc;
/// use std::task::{Context, Poll, Wake, Waker};
///
/// use qubit_io::AsyncDigestOutput;
/// use qubit_io::AsyncOutput;
///
/// struct RecordingOutput {
///     bytes: Vec<u8>,
///     flushed: bool,
/// }
///
/// impl AsyncOutput for RecordingOutput {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         input: &[u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<io::Result<usize>> {
///         self.get_mut().bytes.extend_from_slice(&input[index..index + count]);
///         Poll::Ready(Ok(count))
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
///         self.get_mut().flushed = true;
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// struct NoopWake;
/// impl Wake for NoopWake {
///     fn wake(self: Arc<Self>) {}
/// }
///
/// let waker = Waker::from(Arc::new(NoopWake));
/// let mut cx = Context::from_waker(&waker);
/// let mut output = AsyncDigestOutput::new(
///     RecordingOutput { bytes: Vec::new(), flushed: false },
///     Sha256::new(),
/// );
/// let operation = async {
///     output.write_fully_async(b"abc").await?;
///     assert_eq!(output.digest(), Sha256::digest(b"abc"));
///     output.flush_async().await?;
///     let (inner, digest) = output.finalize();
///     assert!(inner.flushed);
///     assert_eq!(inner.bytes, b"abc");
///     assert_eq!(digest, Sha256::digest(b"abc"));
///     io::Result::Ok(())
/// };
/// let mut operation = std::pin::pin!(operation);
/// match operation.as_mut().poll(&mut cx) {
///     Poll::Ready(Ok(())) => {}
///     Poll::Ready(Err(error)) => panic!("I/O failed: {error}"),
///     Poll::Pending => panic!("the recording output must always be ready"),
/// }
/// ```
///
/// # Type Parameters
///
/// - `O`: Wrapped asynchronous byte output type.
/// - `D`: Digest hasher type.
#[must_use]
#[derive(Debug)]
pub struct AsyncDigestOutput<O, D> {
    /// Output whose successful writes are hashed.
    inner: O,
    // Only `inner` is structurally pinned; `hasher` is never projected as Pin.
    /// Digest state tracking accepted bytes.
    hasher: D,
}

impl<O, D> AsyncClose for AsyncDigestOutput<O, D>
where
    O: AsyncClose<Item = u8>,
    D: Digest,
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
        // SAFETY: Only `inner` is structurally pinned and it is never moved.
        // `hasher` is not structurally pinned and is accessed without Pin.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}

impl<O, D> AsyncDigestOutput<O, D>
where
    D: Digest,
{
    /// Creates a digest-tracking asynchronous output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous byte output to wrap.
    /// - `hasher`: Digest state updated after successful writes.
    ///
    /// # Returns
    ///
    /// Returns an output with the supplied initial hasher state.
    #[inline(always)]
    pub const fn new(inner: O, hasher: D) -> Self {
        Self { inner, hasher }
    }

    /// Returns a snapshot of the current digest without changing its state.
    ///
    /// Clones the algorithm state and finalizes the clone. Further transfers
    /// continue accumulating into the original state.
    ///
    /// # Returns
    ///
    /// Returns the algorithm's full, fixed-size digest of bytes tracked so far.
    #[inline]
    #[must_use]
    pub fn digest(&self) -> digest::Output<D>
    where
        D: Clone,
    {
        self.hasher.clone().finalize()
    }

    /// Consumes this wrapper and finalizes the digest without performing I/O.
    ///
    /// Does not flush buffered output or close the underlying stream. Flush
    /// or close explicitly and check the result before finalizing when
    /// successful delivery must be confirmed. The digest covers accepted bytes,
    /// including bytes still buffered by the underlying output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped stream and the algorithm's full, fixed-size digest.
    #[inline]
    #[must_use]
    pub fn finalize(self) -> (O, digest::Output<D>) {
        (self.inner, self.hasher.finalize())
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// Writes performed directly on the returned output do not update this
    /// wrapper's hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte output.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Returns a shared reference to the hasher.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state.
    #[inline(always)]
    #[must_use]
    pub const fn hasher(&self) -> &D {
        &self.hasher
    }

    /// Returns a mutable reference to the hasher.
    ///
    /// Mutating the returned hasher changes the digest independently of
    /// bytes written through this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state mutably.
    #[inline(always)]
    #[must_use]
    pub fn hasher_mut(&mut self) -> &mut D {
        &mut self.hasher
    }

    /// Consumes this wrapper without flushing the wrapped output.
    ///
    /// This method does not call [`AsyncOutput::flush_async`] and performs no
    /// asynchronous I/O. Any buffering owned by the returned output remains
    /// pending and unchanged.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and final hasher state.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (O, D) {
        (self.inner, self.hasher)
    }
}

impl<O, D> AsyncOutput for AsyncDigestOutput<O, D>
where
    O: AsyncByteOutput,
    D: Digest,
{
    /// Byte item hashed after successful writes.
    type Item = u8;

    /// Preserves the wrapped output's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a write and hashes only bytes in a successful ready result.
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
    /// Returns [`Poll::Pending`] when the output is not ready. A ready success
    /// contains the number of bytes accepted and hashed.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped output without changing the
    /// hasher.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] without updating
    /// the hasher.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: Only `inner` is structurally pinned and it is never moved.
        // `hasher` is not structurally pinned and is accessed without Pin.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let source = &input[index..index + count];
        let requested = source.len();
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_write(cx, source) {
            Poll::Ready(Ok(written)) => {
                if let Err(error) = validate_write_count(written, requested) {
                    return Poll::Ready(Err(error));
                }
                if written != 0 {
                    this.hasher.update(&input[index..index + written]);
                }
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
        // SAFETY: Only `inner` is structurally pinned and it is never moved.
        // `hasher` is not structurally pinned and is accessed without Pin.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
