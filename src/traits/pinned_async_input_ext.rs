// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::AsyncInput;
use crate::ReadExactFuture;
use crate::ReadFullyFuture;
use crate::ReadFuture;

/// Convenience futures for an already pinned asynchronous input.
///
/// This extension serves inputs that are not Unpin and pinned input trait
/// objects, for which AsyncInput convenience methods taking a movable mutable
/// reference are unavailable.
///
/// # Examples
///
/// ```
/// use qubit_io::{AsyncInput, PinnedAsyncInputExt};
/// use std::future::Future;
/// use std::io;
/// use std::pin::Pin;
/// use std::sync::Arc;
/// use std::task::{Context, Poll, Wake, Waker};
///
/// struct ReadyInput {
///     bytes: &'static [u8],
///     position: usize,
/// }
///
/// impl AsyncInput for ReadyInput {
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
///         let available = this.bytes.len().saturating_sub(this.position);
///         let read = count.min(available);
///         output[index..index + read]
///             .copy_from_slice(&this.bytes[this.position..this.position + read]);
///         this.position += read;
///         Poll::Ready(Ok(read))
///     }
/// }
///
/// struct NoopWake;
/// impl Wake for NoopWake {
///     fn wake(self: Arc<Self>) {}
/// }
///
/// let mut input: Pin<Box<dyn AsyncInput<Item = u8>>> = Box::pin(ReadyInput {
///     bytes: b"abc",
///     position: 0,
/// });
/// let waker = Waker::from(Arc::new(NoopWake));
/// let mut cx = Context::from_waker(&waker);
/// let operation = async {
///     let mut bytes = [0; 3];
///     let mut pinned = input.as_mut();
///     pinned.read_exactly_async(&mut bytes).await?;
///     assert_eq!(&bytes, b"abc");
///     io::Result::Ok(())
/// };
/// let mut operation = std::pin::pin!(operation);
/// match operation.as_mut().poll(&mut cx) {
///     Poll::Ready(Ok(())) => {}
///     Poll::Ready(Err(error)) => panic!("I/O failed: {error}"),
///     Poll::Pending => panic!("the input must always be ready"),
/// }
/// ```
pub trait PinnedAsyncInputExt {
    /// The pinned asynchronous input type.
    type Input: AsyncInput + ?Sized;

    /// Creates a future that performs one read.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with the input error if a read fails, or `InvalidData` if the
    /// input reports more items than requested.
    fn read_async<'a>(&'a mut self, output: &'a mut [<Self::Input as AsyncInput>::Item])
    -> ReadFuture<'a, Self::Input>;

    /// Creates a future that reads until the destination is full or EOF.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with the input error if a read fails, or `InvalidData` if the
    /// input reports more items than requested.
    fn read_fully_async<'a>(
        &'a mut self,
        output: &'a mut [<Self::Input as AsyncInput>::Item],
    ) -> ReadFullyFuture<'a, Self::Input>;

    /// Creates a future that fills the entire destination.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with `UnexpectedEof` if EOF occurs before `output` is full,
    /// with the input error if a read fails, or with `InvalidData` if the
    /// input reports more items than requested.
    fn read_exactly_async<'a>(
        &'a mut self,
        output: &'a mut [<Self::Input as AsyncInput>::Item],
    ) -> ReadExactFuture<'a, Self::Input>;
}

impl<I> PinnedAsyncInputExt for Pin<&mut I>
where
    I: AsyncInput + ?Sized,
{
    /// The asynchronous input behind this pinned reference.
    type Input = I;

    /// Creates a single-read future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with the input error if a read fails, or `InvalidData` if the
    /// input reports more items than requested.
    #[inline(always)]
    fn read_async<'a>(&'a mut self, output: &'a mut [I::Item]) -> ReadFuture<'a, Self::Input> {
        ReadFuture::new(self.as_mut(), output)
    }

    /// Creates a read-fully future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with the input error if a read fails, or `InvalidData` if the
    /// input reports more items than requested.
    #[inline(always)]
    fn read_fully_async<'a>(&'a mut self, output: &'a mut [I::Item]) -> ReadFullyFuture<'a, Self::Input> {
        ReadFullyFuture::new(self.as_mut(), output)
    }

    /// Creates an exact-read future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
    /// Resolves with `UnexpectedEof` if EOF occurs before `output` is full,
    /// with the input error if a read fails, or with `InvalidData` if the
    /// input reports more items than requested.
    #[inline(always)]
    fn read_exactly_async<'a>(&'a mut self, output: &'a mut [I::Item]) -> ReadExactFuture<'a, Self::Input> {
        ReadExactFuture::new(self.as_mut(), output)
    }
}
