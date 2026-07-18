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
    AsyncInput,
    Buffer,
    buffered::DEFAULT_BUFFER_CAPACITY,
};

/// Buffered asynchronous item input.
///
/// This wrapper preserves unread items across [`Poll::Pending`] and performs
/// no runtime-specific work. The wrapped input can be `!Unpin`; pin projection
/// is kept internal and the input is never moved while this wrapper is pinned.
#[derive(Debug)]
pub struct AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Copy + Default,
{
    inner: I,
    buffer: Buffer<I::Item>,
}

impl<I> AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Copy + Default,
{
    /// Creates a buffered input with the default item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item input to buffer.
    ///
    /// # Returns
    ///
    /// Returns a buffered input with [`DEFAULT_BUFFER_CAPACITY`] items.
    #[must_use]
    pub fn new(inner: I) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered input with a requested item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item input to buffer.
    /// - `capacity`: Requested number of buffered items.
    ///
    /// # Returns
    ///
    /// Returns a buffered input whose actual capacity is at least one.
    #[must_use]
    pub fn with_capacity(inner: I, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped input.
    ///
    /// Direct reads can invalidate the logical position represented by unread
    /// buffered items.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the input and discards unread buffered items.
    #[must_use]
    pub fn into_inner(self) -> I {
        self.inner
    }

    /// Consumes this wrapper and preserves its unread buffer.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and buffer. Unread items remain in the
    /// buffer's readable window.
    #[must_use]
    pub fn into_parts(self) -> (I, Buffer<I::Item>) {
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

    /// Returns the number of unread buffered items.
    ///
    /// # Returns
    ///
    /// Returns the readable-window length.
    #[must_use]
    pub const fn unread_len(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the unread buffered item window.
    ///
    /// # Returns
    ///
    /// Returns items already fetched but not yet returned to the caller.
    #[must_use]
    pub fn unread(&self) -> &[I::Item] {
        self.buffer.readable()
    }
}

impl<I> AsyncInput for AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Copy + Default,
{
    type Item = I::Item;

    /// Reports that this input already buffers items.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Polls one read through the retained item buffer.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }

        // SAFETY: `inner` is never moved after projecting from this pinned
        // wrapper. `buffer` does not structurally pin any value.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if !this.buffer.is_empty() {
            let read = count.min(this.buffer.available());
            output[index..index + read]
                .copy_from_slice(&this.buffer.readable()[..read]);
            // SAFETY: `read` was bounded by the readable-window length.
            unsafe {
                this.buffer.consume(read);
            }
            return Poll::Ready(Ok(read));
        }

        this.buffer.clear();
        if count >= this.buffer.capacity() {
            let destination = &mut output[index..index + count];
            // SAFETY: The pinned wrapper never moves `inner`.
            let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
            return inner.poll_read(cx, destination);
        }

        let result = {
            // SAFETY: The pinned wrapper never moves `inner`.
            let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
            inner.poll_read(cx, this.buffer.data_mut())
        };
        match result {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
            Poll::Ready(Ok(fetched)) => {
                // SAFETY: `AsyncInput::poll_read` validated `fetched` against
                // the complete backing-buffer length.
                unsafe {
                    this.buffer.advance(fetched);
                }
                let read = count.min(fetched);
                output[index..index + read]
                    .copy_from_slice(&this.buffer.readable()[..read]);
                // SAFETY: `read <= fetched` items are readable.
                unsafe {
                    this.buffer.consume(read);
                }
                Poll::Ready(Ok(read))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}
