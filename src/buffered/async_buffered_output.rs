// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::TryReserveError;
use std::future::poll_fn;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::AsyncClose;
use crate::AsyncOutput;
use crate::Buffer;
use crate::async_io::MAX_READY_OPERATIONS_PER_POLL;
use crate::buffered::DEFAULT_BUFFER_CAPACITY;
use crate::traits::normalize_async_error;

/// Buffered asynchronous item output.
///
/// Accepted items remain owned by this wrapper until the inner output accepts
/// them. Partial writes are committed before another [`Poll::Pending`], making
/// flushing cancellation-safe. Dropping this type cannot perform asynchronous
/// I/O; callers that need delivery guarantees must poll `flush` to completion
/// or recover the pending buffer through [`Self::into_parts`].
///
/// # Type Parameters
///
/// - `O`: Asynchronous item output type.
#[must_use]
#[derive(Debug)]
pub struct AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Clone + Default,
{
    /// Asynchronous output receiving buffered items.
    inner: O,
    /// Storage retaining accepted but undelivered items.
    buffer: Buffer<O::Item>,
}

impl<O> AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Clone + Default,
{
    /// Creates a buffered output with the default item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item output to buffer.
    ///
    /// # Returns
    ///
    /// Returns a buffered output with [`DEFAULT_BUFFER_CAPACITY`] items.
    ///
    /// # Panics
    ///
    /// Panics if `O::Item::default()` or `O::Item::clone()` panics, or the
    /// default backing length exceeds [`Vec`]'s supported capacity.
    #[inline(always)]
    pub fn new(inner: O) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered output with a requested item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item output to buffer.
    /// - `capacity`: Requested number of buffered items.
    ///
    /// # Returns
    ///
    /// Returns a buffered output whose actual capacity is at least one.
    ///
    /// # Panics
    ///
    /// Panics if `O::Item::default()` or `O::Item::clone()` panics, or the
    /// requested backing length exceeds [`Vec`]'s supported capacity.
    #[inline]
    pub fn with_capacity(inner: O, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Tries to create a buffered output with a requested item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item output to buffer.
    /// - `capacity`: Requested number of buffered items.
    ///
    /// # Returns
    ///
    /// Returns a buffered output whose actual capacity is at least one.
    ///
    /// # Errors
    ///
    /// Returns the allocation error when the backing buffer cannot be
    /// allocated.
    ///
    /// # Panics
    ///
    /// Panics if initializing the backing buffer requires
    /// `O::Item::default()` or `O::Item::clone()` and either operation panics.
    #[inline]
    pub fn try_with_capacity(inner: O, capacity: usize) -> Result<Self, TryReserveError> {
        Ok(Self {
            inner,
            buffer: Buffer::try_with_capacity(capacity)?,
        })
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Items may still be pending in this wrapper.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped output.
    ///
    /// Direct output calls can be ordered before items retained in the buffer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper without flushing the wrapped output.
    ///
    /// This method does not call [`AsyncOutput::flush_async`] and performs no
    /// asynchronous I/O. Call [`AsyncOutput::flush_async`] before this method
    /// for normal completion. Otherwise, the returned buffer contains the
    /// pending items that the caller must write before continuing the logical
    /// stream.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending item buffer.
    #[inline(always)]
    #[must_use = "the returned inner output and pending buffer must be handled"]
    pub fn into_parts(self) -> (O, Buffer<O::Item>) {
        (self.inner, self.buffer)
    }

    /// Returns the internal item capacity.
    ///
    /// # Returns
    ///
    /// Returns the total number of items in the backing buffer.
    #[inline(always)]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Returns the number of pending buffered items.
    ///
    /// # Returns
    ///
    /// Returns the readable-window length awaiting delivery.
    #[inline(always)]
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the pending buffered item window.
    ///
    /// # Returns
    ///
    /// Returns items accepted by this wrapper but not yet accepted by the
    /// inner output.
    #[inline(always)]
    #[must_use]
    pub fn pending(&self) -> &[O::Item] {
        self.buffer.readable()
    }

    /// Tries to ensure that the internal item capacity is at least `capacity`.
    ///
    /// Pending items are retained and this method performs no I/O.
    ///
    /// # Parameters
    ///
    /// - `capacity`: Minimum total item capacity to reserve.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the backing buffer has at least `capacity`
    /// item slots.
    ///
    /// # Errors
    ///
    /// Returns the allocation error when the backing buffer cannot grow.
    ///
    /// # Panics
    ///
    /// Panics if growing the backing buffer requires `O::Item::default()` or
    /// `O::Item::clone()` and either operation panics.
    #[inline(always)]
    pub fn try_reserve_capacity(&mut self, capacity: usize) -> Result<(), TryReserveError> {
        self.buffer.try_reserve_capacity(capacity)
    }

    /// Returns the unused capacity in the internal buffer.
    ///
    /// # Returns
    ///
    /// Returns the number of items that can be buffered without draining.
    #[inline(always)]
    #[must_use]
    pub fn spare_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Returns the full backing storage and its spare-tail range.
    ///
    /// Call [`Self::advance`] after writing initialized items into the returned
    /// spare range.
    ///
    /// # Returns
    ///
    /// Returns the backing storage together with the first and past-the-end
    /// indexes of its spare range.
    #[inline(always)]
    #[must_use]
    pub fn spare_raw_parts_mut(&mut self) -> (&mut [O::Item], usize, usize) {
        self.buffer.spare_raw_parts_mut()
    }

    /// Advances the pending-item limit without checking bounds.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of initialized spare items to mark as pending.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.spare_capacity()` and
    /// that the corresponding spare items have been initialized.
    #[inline(always)]
    pub unsafe fn advance(&mut self, count: usize) {
        // SAFETY: The caller guarantees that initialized items fit the spare
        // tail.
        unsafe {
            self.buffer.advance(count);
        }
    }

    /// Polls delivery of pending items when `count` spare items are needed.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `count`: Minimum number of spare item slots to make available.
    ///
    /// # Returns
    ///
    /// Returns a ready success when the requested spare capacity is available,
    /// or [`Poll::Pending`] while the wrapped output is not ready.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the buffer
    /// capacity, [`ErrorKind::WriteZero`] when draining makes no progress, or
    /// an error reported by the wrapped output.
    pub fn poll_ensure_spare_capacity(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        count: usize,
    ) -> Poll<io::Result<()>> {
        if count > self.as_ref().get_ref().buffer.capacity() {
            return Poll::Ready(Err(Error::new(
                ErrorKind::InvalidInput,
                "requested spare capacity exceeds buffered output capacity",
            )));
        }
        if self.as_ref().get_ref().buffer.spare_capacity() < count {
            return self.as_mut().poll_drain_buffer(cx);
        }
        Poll::Ready(Ok(()))
    }

    /// Polls pending-item delivery without flushing the inner output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while the inner output is not ready, or a
    /// ready success after all pending items are delivered.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::WriteZero`] if the inner output accepts no
    /// pending item. Other errors are propagated from the inner output.
    fn poll_drain_buffer(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved after projecting from this pinned
        // wrapper. `buffer` does not structurally pin any value.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let mut ready_operations = 0;
        while !this.buffer.is_empty() {
            let result = {
                let pending = this.buffer.readable();
                // SAFETY: The pinned wrapper never moves `inner`.
                let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
                inner.poll_write(cx, pending)
            };
            match result {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
                }
                Poll::Ready(Ok(written)) => {
                    // SAFETY: `poll_write` validated the returned count against
                    // the pending slice length.
                    unsafe {
                        this.buffer.consume(written);
                    }
                    ready_operations += 1;
                    if !this.buffer.is_empty() && ready_operations >= MAX_READY_OPERATIONS_PER_POLL {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        this.buffer.clear();
        Poll::Ready(Ok(()))
    }
}

impl<O> AsyncBufferedOutput<O>
where
    O: AsyncOutput + Unpin,
    O::Item: Clone + Default + Unpin,
{
    /// Asynchronously ensures that the pending buffer has room for at least
    /// `count` more items.
    ///
    /// Pending items are written to the wrapped output when necessary; this
    /// does not flush the wrapped output itself.
    ///
    /// # Parameters
    ///
    /// - `count`: Minimum number of spare item slots to make available.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the requested spare capacity is available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the buffer
    /// capacity, [`ErrorKind::WriteZero`] when the wrapped output makes no
    /// progress, or an error from the wrapped output.
    pub async fn ensure_spare_capacity_async(&mut self, count: usize) -> io::Result<()> {
        poll_fn(|cx| Pin::new(&mut *self).poll_ensure_spare_capacity(cx, count)).await
    }
}

impl<O> AsyncOutput for AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Clone + Default,
{
    /// Item type accepted by the wrapped output.
    type Item = O::Item;

    /// Reports that this output already buffers items.
    ///
    /// # Returns
    ///
    /// Always returns `true`.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Polls one write through the retained item buffer.
    ///
    /// A zero-length request completes immediately. The method first uses
    /// spare buffer capacity, then drains pending items when necessary.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to accept.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] when pending items cannot yet be delivered. A
    /// ready success contains the number of newly accepted items.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::WriteZero`] if draining makes no progress.
    /// Other errors are propagated from the wrapped output.
    ///
    /// # Panics
    ///
    /// May panic if a nonzero requested input range does not fit. Debug builds
    /// validate buffered-copy ranges before copying.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }

        // SAFETY: This projection only inspects the unpinned buffer field.
        let (spare, capacity) = unsafe {
            let this = self.as_mut().get_unchecked_mut();
            (this.buffer.spare_capacity(), this.buffer.capacity())
        };
        // Keep exact remaining-space writes buffered, but let a full-capacity
        // write into an empty buffer take the direct path below.
        if count <= spare && count < capacity {
            // SAFETY: The caller guarantees the source range and the branch
            // proves that the destination spare range is large enough.
            unsafe {
                self.as_mut().get_unchecked_mut().buffer.copy_from(input, index, count);
            }
            return Poll::Ready(Ok(count));
        }

        match self.as_mut().poll_drain_buffer(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => return Poll::Pending,
        }

        // SAFETY: `inner` is never moved after projecting from this pinned
        // wrapper.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if count >= capacity {
            let source = &input[index..index + count];
            // SAFETY: The pinned wrapper never moves `inner`.
            let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
            return inner.poll_write(cx, source);
        }

        // SAFETY: Draining cleared the buffer, `count` fits its total
        // capacity, and the caller guarantees the source range.
        unsafe {
            this.buffer.copy_from(input, index, count);
        }
        Poll::Ready(Ok(count))
    }

    /// Polls delivery of pending items followed by the inner flush operation.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while delivery or flushing is incomplete,
    /// otherwise a ready success result.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::WriteZero`] if draining makes no progress, or
    /// an error reported by the wrapped output. Invalid asynchronous error
    /// kinds from the flush operation are normalized to
    /// [`io::ErrorKind::InvalidData`].
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_drain_buffer(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => return Poll::Pending,
        }
        // SAFETY: The pinned wrapper never moves `inner`.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: `inner` remains pinned in place for this call.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_flush(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}

impl<O> AsyncClose for AsyncBufferedOutput<O>
where
    O: AsyncClose,
    O::Item: Clone + Default,
{
    /// Polls delivery of pending items followed by closing the inner output.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns [`Poll::Pending`] while delivery or closing is incomplete,
    /// otherwise a ready success result.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::WriteZero`] if draining makes no progress, or
    /// an error reported by the wrapped output. Invalid asynchronous error
    /// kinds from the close operation are normalized to
    /// [`io::ErrorKind::InvalidData`].
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_drain_buffer(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => return Poll::Pending,
        }
        // SAFETY: The pinned wrapper never moves `inner`.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: `inner` remains pinned in place for this call.
        unsafe { Pin::new_unchecked(&mut this.inner) }
            .poll_close(cx)
            .map(|result| result.map_err(normalize_async_error))
    }
}
