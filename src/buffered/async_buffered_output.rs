// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    io,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use crate::{
    AsyncClose,
    AsyncOutput,
    Buffer,
    buffered::DEFAULT_BUFFER_CAPACITY,
    traits::validate_async_error,
};

/// Buffered asynchronous item output.
///
/// Accepted items remain owned by this wrapper until the inner output accepts
/// them. Partial writes are committed before another [`Poll::Pending`], making
/// flushing cancellation-safe. Dropping this type cannot perform asynchronous
/// I/O; callers that need delivery guarantees must poll `flush` to completion
/// or recover the pending buffer through [`Self::into_parts`].
#[derive(Debug)]
pub struct AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Copy + Default,
{
    inner: O,
    buffer: Buffer<O::Item>,
}

impl<O> AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Copy + Default,
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
    #[must_use]
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
    #[must_use]
    pub fn with_capacity(inner: O, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output. Items may still be pending in this wrapper.
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
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this wrapper without performing asynchronous I/O.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and pending item buffer.
    #[must_use]
    pub fn into_parts(self) -> (O, Buffer<O::Item>) {
        (self.inner, self.buffer)
    }

    /// Returns the internal item capacity.
    ///
    /// # Returns
    ///
    /// Returns the total number of items in the backing buffer.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Returns the number of pending buffered items.
    ///
    /// # Returns
    ///
    /// Returns the readable-window length awaiting delivery.
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
    #[must_use]
    pub fn pending(&self) -> &[O::Item] {
        self.buffer.readable()
    }

    /// Polls pending-item delivery without flushing the inner output.
    fn poll_drain_buffer(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `inner` is never moved after projecting from this pinned
        // wrapper. `buffer` does not structurally pin any value.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
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
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        this.buffer.clear();
        Poll::Ready(Ok(()))
    }
}

impl<O> AsyncOutput for AsyncBufferedOutput<O>
where
    O: AsyncOutput,
    O::Item: Copy + Default,
{
    type Item = O::Item;

    /// Reports that this output already buffers items.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Polls one write through the retained item buffer.
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
        let spare = unsafe {
            self.as_mut().get_unchecked_mut().buffer.spare_capacity()
        };
        if count <= spare {
            // SAFETY: The caller guarantees the source range and the branch
            // proves that the destination spare range is large enough.
            unsafe {
                self.as_mut()
                    .get_unchecked_mut()
                    .buffer
                    .copy_from(input, index, count);
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
        if count > this.buffer.capacity() {
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
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
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
            .map(|result| result.map_err(validate_async_error))
    }
}

impl<O> AsyncClose for AsyncBufferedOutput<O>
where
    O: AsyncClose,
    O::Item: Copy + Default,
{
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
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
            .map(|result| result.map_err(validate_async_error))
    }
}
