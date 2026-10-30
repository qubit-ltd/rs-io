// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::ReadExactFuture;
use crate::ReadFullyFuture;
use crate::ReadFuture;
use crate::traits::normalize_async_error;
use crate::traits::validate_read_count;

/// Minimal runtime-independent asynchronous input interface over items.
///
/// `AsyncInput` expresses readiness through [`Poll`] and does not depend on a
/// particular executor. Implementations may be pinned and therefore are never
/// moved by the polling methods.
///
/// # Examples
///
/// Implement the trait for a small in-memory input and inspect its buffering
/// behavior through the public API:
///
/// ```
/// use std::io::Result;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// use qubit_io::AsyncInput;
///
/// struct SliceInput<'a> {
///     items: &'a [u8],
///     position: usize,
/// }
///
/// impl AsyncInput for SliceInput<'_> {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         output: &mut [Self::Item],
///         index: usize,
///         count: usize,
///     ) -> Poll<Result<usize>> {
///         let this = self.get_mut();
///         let available = this.items.len() - this.position;
///         let read = available.min(count);
///         output[index..index + read]
///             .copy_from_slice(&this.items[this.position..this.position + read]);
///         this.position += read;
///         Poll::Ready(Ok(read))
///     }
/// }
///
/// let mut input = SliceInput {
///     items: b"data",
///     position: 0,
/// };
/// assert!(!input.is_buffered());
/// let mut cx = Context::from_waker(Waker::noop());
/// let mut bytes = [0; 4];
/// assert!(matches!(Pin::new(&mut input).poll_read(&mut cx, &mut bytes),
///                  Poll::Ready(Ok(4))));
/// assert_eq!(b"data", &bytes);
/// assert!(matches!(Pin::new(&mut input).poll_read(&mut cx, &mut bytes),
///                  Poll::Ready(Ok(0))));
/// ```
pub trait AsyncInput {
    /// The item type read from this input.
    type Item;

    /// Returns whether this input already buffers items internally.
    ///
    /// # Returns
    ///
    /// `true` when callers should avoid automatically adding another generic
    /// item buffer.
    #[inline(always)]
    #[must_use]
    fn is_buffered(&self) -> bool {
        false
    }

    /// Polls an indexed read without checking the destination range.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when input is pending.
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] when no result is currently available, or a ready I/O
    /// result containing a count in `0..=count`. A ready zero count denotes
    /// end of input when `count` is nonzero. A zero `count` must immediately
    /// return `Poll::Ready(Ok(0))`.
    ///
    /// Before returning [`Poll::Pending`], the implementation must arrange for
    /// `cx`'s waker to be notified when progress may be possible. Neither
    /// `Poll::Pending` nor `Poll::Ready(Err(_))` may transfer items.
    /// `WouldBlock` and `Interrupted` must not cross this asynchronous
    /// boundary; implementations must respectively register readiness or retry
    /// internally.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>>;

    /// Polls a read into the full destination slice.
    ///
    /// # Parameters
    ///
    /// * `cx` - Task context used to register interest when input is pending.
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] or a ready result containing the number of items read.
    ///
    /// # Errors
    ///
    /// The default implementation converts `WouldBlock` and `Interrupted`
    /// errors from the underlying operation to
    /// [`std::io::ErrorKind::InvalidData`], retaining the original error in
    /// the cause chain. Other errors are returned unchanged. It also
    /// returns `InvalidData` if the implementation reports more items than
    /// requested.
    #[inline]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, output: &mut [Self::Item]) -> Poll<Result<usize>> {
        if output.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let requested = output.len();
        // SAFETY: The full output slice is a valid destination range.
        match unsafe { self.poll_read_unchecked(cx, output, 0, requested) } {
            Poll::Ready(Ok(read)) => Poll::Ready(validate_read_count(read, requested).map(|()| read)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(normalize_async_error(error))),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Creates a future that performs one asynchronous read operation.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the input borrow and destination slice.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of items read.
    ///
    /// # Errors
    ///
    /// The future returns the input error or
    /// [`std::io::ErrorKind::InvalidData`] if the implementation reports more
    /// items than requested.
    #[inline(always)]
    fn read_async<'a>(&'a mut self, output: &'a mut [Self::Item]) -> ReadFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFuture::new(Pin::new(self), output)
    }

    /// Creates a future that fills a destination as far as possible.
    ///
    /// The returned future stops when the destination is full or the input
    /// reports EOF.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the input borrow and destination slice.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the total number of items read.
    ///
    /// # Errors
    ///
    /// The future returns the input error or
    /// [`std::io::ErrorKind::InvalidData`] if the implementation reports more
    /// items than the remaining destination can hold.
    #[inline(always)]
    fn read_fully_async<'a>(&'a mut self, output: &'a mut [Self::Item]) -> ReadFullyFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFullyFuture::new(Pin::new(self), output)
    }

    /// Creates a future that fills the entire destination.
    ///
    /// The returned future reports [`std::io::ErrorKind::UnexpectedEof`] if
    /// the input ends before the destination is full.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the input borrow and destination slice.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage that must be filled completely.
    ///
    /// # Returns
    ///
    /// A future that resolves after filling `output` or encountering an error.
    ///
    /// # Errors
    ///
    /// The future returns [`std::io::ErrorKind::UnexpectedEof`] if the input
    /// ends before `output` is full. It also returns input errors and
    /// [`std::io::ErrorKind::InvalidData`] if the implementation reports more
    /// items than the remaining destination can hold.
    #[inline(always)]
    fn read_exactly_async<'a>(&'a mut self, output: &'a mut [Self::Item]) -> ReadExactFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadExactFuture::new(Pin::new(self), output)
    }
}
