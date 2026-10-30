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
use digest::Output;

use crate::AsyncByteInput;
use crate::AsyncInput;
use crate::DigestAllFuture;
use crate::traits::validate_read_count;

/// Asynchronous byte input that computes a digest of successfully returned
/// bytes.
///
/// Available with the `digest` feature.
///
/// Pending and failed reads do not change the hasher. The algorithm is
/// supplied by the caller. The digest covers transferred bytes in order,
/// independently of transfer chunk boundaries.
///
/// # Examples
///
/// ```
/// use std::future::Future;
/// use std::io;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// use qubit_io::AsyncDigestInput;
/// use qubit_io::AsyncInput;
/// use sha2::Digest;
/// use sha2::Sha256;
///
/// struct SliceInput {
///     bytes: &'static [u8],
///     position: usize,
/// }
///
/// impl AsyncInput for SliceInput {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         output: &mut [u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<io::Result<usize>> {
///         let this = self.get_mut();
///         let count = count.min(this.bytes.len() - this.position);
///         output[index..index + count]
///             .copy_from_slice(&this.bytes[this.position..this.position + count]);
///         this.position += count;
///         Poll::Ready(Ok(count))
///     }
/// }
///
/// let mut input = AsyncDigestInput::new(
///     SliceInput {
///         bytes: b"abc",
///         position: 0,
///     },
///     Sha256::new(),
/// );
/// let mut output = [0; 3];
/// let waker = Waker::noop();
/// let mut cx = Context::from_waker(waker);
/// let mut read = input.read_async(&mut output);
/// assert!(matches!(Pin::new(&mut read).poll(&mut cx), Poll::Ready(Ok(3))));
/// assert_eq!(&output, b"abc");
/// assert_eq!(input.digest(), Sha256::digest(b"abc"));
/// ```
///
/// # Type Parameters
///
/// - `I`: Wrapped asynchronous byte input type.
/// - `D`: Digest hasher type.
#[must_use]
#[derive(Debug)]
pub struct AsyncDigestInput<I, D> {
    /// Input whose successful reads are hashed.
    inner: I,
    // Only `inner` is structurally pinned; `hasher` is never projected as Pin.
    /// Digest state tracking returned bytes.
    hasher: D,
}

impl<I, D> AsyncDigestInput<I, D>
where
    D: Digest,
{
    /// Creates a digest-tracking asynchronous input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous byte input to wrap.
    /// - `hasher`: Digest state updated after successful reads.
    ///
    /// # Returns
    ///
    /// Returns an input with the supplied initial hasher state.
    #[inline(always)]
    pub const fn new(inner: I, hasher: D) -> Self {
        Self { inner, hasher }
    }

    /// Creates a future that reads to EOF and computes a fresh digest.
    ///
    /// Reads from the current position with a new algorithm state and a fixed
    /// bulk-read buffer. Supports pinned `!Unpin` inputs without an executor
    /// dependency. Dropping the future preserves the consumed input position
    /// and discards its partial digest.
    ///
    /// # Parameters
    ///
    /// - `input`: Pinned byte input to consume from its current position.
    ///
    /// # Returns
    ///
    /// Returns a future yielding the full digest of the remaining bytes.
    ///
    /// # Errors
    ///
    /// The future returns the input's read error, including invalid counts,
    /// without rewinding input or returning a partial digest.
    #[inline]
    pub fn digest_all(input: Pin<&mut I>) -> DigestAllFuture<'_, I, D>
    where
        I: AsyncByteInput,
    {
        DigestAllFuture::new(input)
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
    pub fn digest(&self) -> Output<D>
    where
        D: Clone,
    {
        self.hasher.clone().finalize()
    }

    /// Consumes this wrapper and finalizes the digest without performing I/O.
    ///
    /// Does not read any remaining input. To digest the complete original
    /// input, start at its beginning and read it to EOF before finalizing.
    ///
    /// # Returns
    ///
    /// Returns the wrapped stream and the algorithm's full, fixed-size digest.
    #[inline]
    #[must_use]
    pub fn finalize(self) -> (I, Output<D>) {
        (self.inner, self.hasher.finalize())
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// Reads performed directly on the returned input do not update this
    /// wrapper's hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte input.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut I {
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
    /// bytes read through this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state mutably.
    #[inline(always)]
    #[must_use]
    pub fn hasher_mut(&mut self) -> &mut D {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns both owned parts.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and final hasher state.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, D) {
        (self.inner, self.hasher)
    }
}

impl<I, D> AsyncInput for AsyncDigestInput<I, D>
where
    I: AsyncByteInput,
    D: Digest,
{
    /// Byte item hashed after successful reads.
    type Item = u8;

    /// Preserves the wrapped input's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a read and hashes only bytes in a successful ready result.
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
    /// Returns [`Poll::Pending`] when the input is not ready. A ready success
    /// contains the number of bytes read and hashed.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input without changing the
    /// hasher.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] without updating
    /// the hasher.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: Only `inner` is structurally pinned and it is never moved.
        // `hasher` is not structurally pinned and is accessed without Pin.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let destination = &mut output[index..index + count];
        let requested = destination.len();
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll_read(cx, destination) {
            Poll::Ready(Ok(read)) => {
                if let Err(error) = validate_read_count(read, requested) {
                    return Poll::Ready(Err(error));
                }
                if read != 0 {
                    this.hasher.update(&output[index..index + read]);
                }
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
