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

use futures_io::AsyncWrite;

use crate::AsyncClose;
use crate::traits::normalize_async_error;
use crate::traits::validate_write_count;

/// Exposes a Qubit byte [`crate::AsyncOutput`] as a futures-io [`AsyncWrite`].
///
/// # Type Parameters
///
/// - `O`: Qubit byte output type.
///
/// # Examples
///
/// ```
/// use qubit_io::AsyncClose;
/// use qubit_io::AsyncOutput;
/// use qubit_io::FuturesAsyncWrite;
/// use futures_io::AsyncWrite;
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Wake;
/// use std::sync::Arc;
///
/// struct Sink;
///
/// impl AsyncOutput for Sink {
///     type Item = u8;
///
///     unsafe fn poll_write_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         input: &[u8],
///         _index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         Poll::Ready(Ok(count.min(input.len())))
///     }
///
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// impl AsyncClose for Sink {
///     fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
///         Poll::Ready(Ok(()))
///     }
/// }
///
/// struct NoopWake;
///
/// impl Wake for NoopWake {
///     fn wake(self: Arc<Self>) {}
/// }
///
/// let mut writer = FuturesAsyncWrite::new(Sink);
/// let waker = std::task::Waker::from(Arc::new(NoopWake));
/// let mut context = Context::from_waker(&waker);
/// assert!(matches!(
///     AsyncWrite::poll_write(Pin::new(&mut writer), &mut context, b"data"),
///     Poll::Ready(Ok(4)),
/// ));
/// ```
#[must_use]
#[repr(transparent)]
pub struct FuturesAsyncWrite<O> {
    /// Qubit byte output exposed through futures-io.
    inner: O,
}

impl<O> FuturesAsyncWrite<O> {
    /// Creates a futures-io writer around a Qubit byte output.
    ///
    /// # Parameters
    ///
    /// - `inner`: Qubit byte output to expose.
    ///
    /// # Returns
    ///
    /// Returns a futures-io writer adapter that owns `inner`.
    #[inline(always)]
    pub const fn new(inner: O) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit output.
    #[inline(always)]
    #[must_use]
    pub const fn get_ref(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped Qubit output with mutable access.
    #[inline(always)]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Projects a pinned adapter to its pinned wrapped output.
    ///
    /// # Returns
    ///
    /// Returns a pinned mutable reference to the wrapped Qubit output without
    /// moving it.
    #[inline(always)]
    #[must_use]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut O> {
        // SAFETY: The projection does not move `inner`, and the transparent
        // adapter never exposes a way to replace a pinned inner value.
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// Consumes the adapter and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the owned Qubit output.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> AsyncWrite for FuturesAsyncWrite<O>
where
    O: AsyncClose<Item = u8>,
{
    /// Polls one futures-io write through the wrapped Qubit output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source byte slice.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the output is not ready. A ready result
    /// contains the number of bytes accepted.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped output.
    /// Returns [`std::io::ErrorKind::InvalidData`] if the output reports more
    /// bytes than requested, or reports `WouldBlock` or `Interrupted`. These
    /// checks cannot roll back output side effects that already occurred.
    #[inline(always)]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, input: &[u8]) -> Poll<std::io::Result<usize>> {
        let requested = input.len();
        self.get_pin_mut().poll_write(cx, input).map(|result| {
            let written = result.map_err(normalize_async_error)?;
            validate_write_count(written, requested)?;
            Ok(written)
        })
    }

    /// Polls flushing through the wrapped Qubit output.
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
    /// Returns an I/O error reported by the wrapped output. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_pin_mut()
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }

    /// Polls closing through the wrapped Qubit output.
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
    /// Returns an I/O error reported by the wrapped output. Invalid
    /// asynchronous error kinds are normalized to
    /// [`std::io::ErrorKind::InvalidData`].
    #[inline(always)]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_pin_mut()
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
