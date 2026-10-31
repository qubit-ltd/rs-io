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

use crate::AsyncByteOutput;
use crate::AsyncClose;
use crate::AsyncOutput;
use crate::traits::normalize_async_error;
use crate::traits::validate_write_count;

/// Asynchronous byte output that hashes successfully accepted bytes.
///
/// Pending and failed writes do not change the hasher. The checksum algorithm
/// and stability guarantees are those of the supplied [`Hasher`].
/// Each successful non-empty written prefix is passed to [`Hasher::write`] as
/// a separate chunk. Chunk boundaries are preserved, and different chunkings
/// are equivalent only when the supplied hasher guarantees that property.
/// Empty writes do not update the hasher. Use a digest API for a content
/// fingerprint independent of write boundaries.
///
/// # Type Parameters
///
/// - `O`: Wrapped asynchronous byte output type.
/// - `H`: Checksum hasher type.
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
/// use qubit_io::AsyncChecksumOutput;
/// use qubit_io::AsyncOutput;
///
/// #[derive(Default)]
/// struct VecOutput(Vec<u8>);
///
/// impl AsyncOutput for VecOutput {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         input: &[u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<io::Result<usize>> {
///         // SAFETY: This type has no address-sensitive fields.
///         let this = unsafe { self.get_unchecked_mut() };
///         this.0.extend_from_slice(&input[index..index + count]);
///         Poll::Ready(Ok(count))
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// let mut output = AsyncChecksumOutput::new(VecOutput::default(), DefaultHasher::new());
/// let waker = std::task::Waker::noop();
/// let mut cx = Context::from_waker(waker);
/// assert!(matches!(Pin::new(&mut output).poll_write(&mut cx, b"abc"), Poll::Ready(Ok(3))));
/// assert!(matches!(Pin::new(&mut output).poll_flush(&mut cx), Poll::Ready(Ok(()))));
/// assert_eq!(output.inner().0, b"abc");
/// let mut expected = DefaultHasher::new();
/// expected.write(b"abc");
/// assert_eq!(output.checksum(), expected.finish());
/// ```
#[must_use]
#[derive(Debug)]
pub struct AsyncChecksumOutput<O, H> {
    /// Output whose successful writes are hashed.
    inner: O,
    /// Hasher tracking accepted bytes.
    hasher: H,
}

impl<O, H> AsyncClose for AsyncChecksumOutput<O, H>
where
    O: AsyncClose<Item = u8>,
    H: Hasher,
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
    #[inline(always)]
    pub const fn new(inner: O, hasher: H) -> Self {
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

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous byte output.
    #[inline(always)]
    #[must_use = "inspect or use the wrapped output"]
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
    #[must_use = "inspect or use the wrapped output"]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Returns a shared reference to the hasher.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state.
    #[inline(always)]
    #[must_use = "inspect or use the hasher state"]
    pub const fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns a mutable reference to the hasher.
    ///
    /// Mutating the returned hasher changes the checksum independently of
    /// bytes written through this wrapper.
    ///
    /// # Returns
    ///
    /// Returns the current hasher state mutably.
    #[inline(always)]
    #[must_use = "inspect or use the hasher state"]
    pub fn hasher_mut(&mut self) -> &mut H {
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
    #[must_use = "use the returned output and hasher state"]
    pub fn into_parts(self) -> (O, H) {
        (self.inner, self.hasher)
    }
}

impl<O, H> AsyncOutput for AsyncChecksumOutput<O, H>
where
    O: AsyncByteOutput,
    H: Hasher,
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
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
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
                    this.hasher.write(&input[index..index + written]);
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
        // SAFETY: `inner` is never moved while projecting this pinned wrapper.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: The pinned wrapper keeps `inner` at a stable address.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
