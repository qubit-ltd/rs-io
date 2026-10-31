// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::hash::Hasher;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncByteInput;
use crate::AsyncInput;
use crate::traits::validate_read_count;

/// Asynchronous byte input that hashes successfully returned bytes.
///
/// Pending and failed reads do not change the hasher. The checksum algorithm
/// and stability guarantees are those of the supplied [`Hasher`].
/// Each successful non-empty read prefix is passed to [`Hasher::write`] as a
/// separate chunk. Chunk boundaries are preserved, and different chunkings
/// are equivalent only when the supplied hasher guarantees that property.
/// Empty reads do not update the hasher. Use a digest API for a content
/// fingerprint independent of read boundaries.
///
/// # Examples
///
/// ```
/// use std::collections::hash_map::DefaultHasher;
/// use std::hash::Hasher;
/// use std::io;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
///
/// use qubit_io::AsyncChecksumInput;
/// use qubit_io::AsyncInput;
///
/// struct SliceInput { bytes: &'static [u8], position: usize }
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
///         // SAFETY: This type has no address-sensitive fields.
///         let this = unsafe { self.get_unchecked_mut() };
///         let count = count.min(this.bytes.len() - this.position);
///         output[index..index + count].copy_from_slice(&this.bytes[this.position..this.position + count]);
///         this.position += count;
///         Poll::Ready(Ok(count))
///     }
/// }
///
/// let mut input = AsyncChecksumInput::new(SliceInput { bytes: b"abc", position: 0 }, DefaultHasher::new());
/// let waker = std::task::Waker::noop();
/// let mut cx = Context::from_waker(waker);
/// let mut bytes = [0; 3];
/// assert!(matches!(Pin::new(&mut input).poll_read(&mut cx, &mut bytes), Poll::Ready(Ok(3))));
/// assert_eq!(&bytes, b"abc");
/// let mut expected = DefaultHasher::new();
/// expected.write(b"abc");
/// assert_eq!(input.checksum(), expected.finish());
/// ```
///
/// # Type Parameters
///
/// - `I`: Wrapped asynchronous byte input type.
/// - `H`: Checksum hasher type.
#[must_use]
#[derive(Debug)]
pub struct AsyncChecksumInput<I, H> {
    /// Input whose successful reads are hashed.
    inner: I,
    /// Hasher tracking returned bytes.
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
    #[inline(always)]
    pub const fn new(inner: I, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    ///
    /// # Returns
    ///
    /// Returns [`Hasher::finish`] for the current state.
    #[inline(always)]
    #[must_use]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte input.
    #[inline(always)]
    #[must_use = "inspect the wrapped input"]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a shared reference to the hasher.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state.
    #[inline(always)]
    #[must_use = "inspect the current hasher state"]
    pub const fn hasher(&self) -> &H {
        &self.hasher
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
    #[must_use = "use the returned reference to access the wrapped input"]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a mutable reference to the hasher.
    ///
    /// Mutating the returned hasher changes the checksum independently of
    /// bytes read through this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state mutably.
    #[inline(always)]
    #[must_use = "use the returned reference to access the hasher"]
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns both owned parts.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and final hasher state.
    #[inline(always)]
    #[must_use = "retain the wrapped input and final hasher state"]
    pub fn into_parts(self) -> (I, H) {
        (self.inner, self.hasher)
    }
}

impl<I, H> AsyncInput for AsyncChecksumInput<I, H>
where
    I: AsyncByteInput,
    H: Hasher,
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
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
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
                    this.hasher.write(&output[index..index + read]);
                }
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
