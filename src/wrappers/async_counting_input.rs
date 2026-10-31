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

use crate::AsyncByteInput;
use crate::AsyncInput;
use crate::traits::validate_read_count;

/// Asynchronous input that counts successfully returned items.
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
/// use qubit_io::AsyncCountingInput;
/// use qubit_io::AsyncInput;
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
/// let mut input = AsyncCountingInput::new(SliceInput {
///     bytes: b"abc",
///     position: 0,
/// });
/// let mut output = [0; 3];
/// let waker = Waker::noop();
/// let mut cx = Context::from_waker(waker);
/// let mut read = input.read_async(&mut output);
/// assert!(matches!(Pin::new(&mut read).poll(&mut cx), Poll::Ready(Ok(3))));
/// assert_eq!(&output, b"abc");
/// assert_eq!(input.items_read(), 3);
/// ```
///
/// # Type Parameters
///
/// - `I`: Wrapped asynchronous input type.
#[must_use]
#[derive(Debug)]
pub struct AsyncCountingInput<I> {
    /// Input whose successful reads are counted.
    inner: I,
    /// Saturating count of returned items.
    items_read: u64,
}

impl<I> AsyncCountingInput<I> {
    /// Creates a counting asynchronous input.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous input to wrap.
    ///
    /// # Returns
    ///
    /// Returns an input whose item count starts at zero.
    #[inline(always)]
    pub const fn new(inner: I) -> Self {
        Self { inner, items_read: 0 }
    }

    /// Returns the successfully returned item count.
    ///
    /// # Returns
    ///
    /// Returns a count that saturates at [`u64::MAX`].
    #[inline(always)]
    #[must_use]
    pub const fn items_read(&self) -> u64 {
        self.items_read
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
    #[must_use = "inspect the wrapped input"]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// Reads performed directly on the returned input are not included in this
    /// wrapper's item count.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
    #[must_use = "use the returned reference to access the wrapped input"]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the asynchronous input.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> AsyncCountingInput<I>
where
    I: AsyncByteInput,
{
    /// Returns the successfully returned byte count.
    ///
    /// # Returns
    ///
    /// Returns the same value as [`Self::items_read`].
    #[inline(always)]
    #[must_use]
    pub const fn bytes_read(&self) -> u64 {
        self.items_read
    }
}

impl<I> AsyncInput for AsyncCountingInput<I>
where
    I: AsyncInput,
{
    /// Item type counted after successful reads.
    type Item = I::Item;

    /// Preserves the wrapped input's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input's buffering declaration.
    #[inline]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Polls a read and counts only a successful ready result.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when the input is not ready. A ready success
    /// contains the number of items read and added to the saturating count.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input without changing the
    /// count.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] without changing
    /// the counter.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
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
                let read_u64 = u64::try_from(read).unwrap_or(u64::MAX);
                this.items_read = this.items_read.saturating_add(read_u64);
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
