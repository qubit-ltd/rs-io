// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tokio::io::AsyncRead;
use tokio::io::ReadBuf;

use crate::AsyncByteInput;
use crate::traits::normalize_async_error;
use crate::traits::validate_read_count;

/// Exposes a Qubit [`AsyncByteInput`] as a Tokio [`AsyncRead`].
///
/// # Type Parameters
///
/// - `I`: Qubit byte input type.
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncInput;
/// use qubit_io::TokioAsyncRead;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Wake;
/// use std::task::Waker;
/// use std::sync::Arc;
/// use tokio::io::AsyncRead;
/// use tokio::io::ReadBuf;
///
/// struct EmptyInput;
///
/// impl AsyncInput for EmptyInput {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         _output: &mut [u8],
///         _index: usize,
///         _count: usize,
///     ) -> Poll<Result<usize>> {
///         Poll::Ready(Ok(0))
///     }
/// }
///
/// struct NoopWake;
///
/// impl Wake for NoopWake {
///     fn wake(self: Arc<Self>) {}
/// }
///
/// let mut reader = TokioAsyncRead::new(EmptyInput);
/// let waker = Waker::from(Arc::new(NoopWake));
/// let mut context = Context::from_waker(&waker);
/// let mut bytes = [0; 1];
/// let mut output = ReadBuf::new(&mut bytes);
/// assert!(matches!(
///     AsyncRead::poll_read(Pin::new(&mut reader), &mut context, &mut output),
///     Poll::Ready(Ok(())),
/// ));
/// assert_eq!(output.filled(), &[]);
/// ```
#[must_use]
#[repr(transparent)]
pub struct TokioAsyncRead<I> {
    /// Qubit byte input exposed through Tokio.
    inner: I,
}

impl<I> TokioAsyncRead<I> {
    /// Creates a Tokio reader around a Qubit byte input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Qubit byte input to expose.
    ///
    /// # Returns
    ///
    /// Returns a Tokio reader adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit input.
    #[inline(always)]
    #[must_use = "the wrapped input reference must be used"]
    pub const fn get_ref(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit input with mutable access.
    #[inline(always)]
    #[must_use = "the mutable wrapped input reference must be used"]
    pub const fn get_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped input.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped Qubit input without
    /// moving it.
    #[inline(always)]
    #[must_use = "the pinned wrapped input reference must be used"]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut I> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the owned Qubit input.
    #[inline(always)]
    #[must_use = "the wrapped input must be used"]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> AsyncRead for TokioAsyncRead<I>
where
    I: AsyncByteInput,
{
    /// Polls one Tokio read through the wrapped Qubit input.
    ///
    /// A destination with no remaining capacity completes immediately without
    /// polling `inner`.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Tokio destination buffer to fill.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready, otherwise a ready
    /// success result after advancing `output`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input.
    /// Returns [`std::io::ErrorKind::InvalidData`] if the input reports more
    /// bytes than requested, or reports `WouldBlock` or `Interrupted`.
    /// Validation errors cannot roll back writes the input already made to
    /// initialized destination storage.
    #[inline]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let requested = output.remaining();
        if requested == 0 {
            return Poll::Ready(Ok(()));
        }
        self.get_pin_mut()
            .poll_read(cx, output.initialize_unfilled())
            .map(|result| {
                let read = result.map_err(normalize_async_error)?;
                validate_read_count(read, requested)?;
                output.advance(read);
                Ok(())
            })
    }
}
