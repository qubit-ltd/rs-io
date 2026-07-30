// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    collections::TryReserveError,
    future::poll_fn,
    io::{
        self,
        Error,
        ErrorKind,
    },
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use crate::{
    AsyncInput,
    Buffer,
    UncheckedSlice,
    async_io::MAX_READY_OPERATIONS_PER_POLL,
    buffered::DEFAULT_BUFFER_CAPACITY,
};

/// Buffered asynchronous item input.
///
/// This wrapper preserves unread items across [`Poll::Pending`] and performs
/// no runtime-specific work. The wrapped input can be `!Unpin`; pin projection
/// is kept internal and the input is never moved while this wrapper is pinned.
///
/// # Type Parameters
///
/// - `I`: Asynchronous item input type.
#[must_use]
#[derive(Debug)]
pub struct AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Clone + Default,
{
    /// Asynchronous input being buffered.
    inner: I,
    /// Storage retaining fetched but unread items.
    buffer: Buffer<I::Item>,
}

impl<I> AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Clone + Default,
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
    ///
    /// # Panics
    ///
    /// Panics if `I::Item::default()` or `I::Item::clone()` panics, or the
    /// default backing length exceeds [`Vec`]'s supported capacity.
    #[inline(always)]
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
    ///
    /// # Panics
    ///
    /// Panics if `I::Item::default()` or `I::Item::clone()` panics, or the
    /// requested backing length exceeds [`Vec`]'s supported capacity.
    #[inline]
    pub fn with_capacity(inner: I, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Tries to create a buffered input with a requested item capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Asynchronous item input to buffer.
    /// - `capacity`: Requested number of buffered items.
    ///
    /// # Returns
    ///
    /// Returns a buffered input whose actual capacity is at least one.
    ///
    /// # Errors
    ///
    /// Returns the allocation error when the backing buffer cannot be
    /// allocated.
    ///
    /// # Panics
    ///
    /// Panics if initializing the backing buffer requires
    /// `I::Item::default()` or `I::Item::clone()` and either operation panics.
    #[inline]
    pub fn try_with_capacity(
        inner: I,
        capacity: usize,
    ) -> Result<Self, TryReserveError> {
        Ok(Self {
            inner,
            buffer: Buffer::try_with_capacity(capacity)?,
        })
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped asynchronous input.
    #[inline(always)]
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
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this wrapper and preserves its unread buffer without I/O.
    ///
    /// The returned input can be physically ahead of the returned buffer. To
    /// continue the same logical stream, consume the buffer's readable window
    /// before reading from that input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and buffer. Unread items remain in the
    /// buffer's readable window.
    #[inline(always)]
    #[must_use = "the returned inner input and unread buffer must be handled"]
    pub fn into_parts(self) -> (I, Buffer<I::Item>) {
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

    /// Returns the number of unread buffered items.
    ///
    /// # Returns
    ///
    /// Returns the readable-window length.
    #[inline(always)]
    #[must_use]
    pub const fn unread_len(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the unread buffered item window.
    ///
    /// # Returns
    ///
    /// Returns items already fetched but not yet returned to the caller.
    #[inline(always)]
    #[must_use]
    pub fn unread(&self) -> &[I::Item] {
        self.buffer.readable()
    }

    /// Tries to ensure that the internal item capacity is at least `capacity`.
    ///
    /// Existing unread items are retained and this method performs no I/O.
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
    /// Panics if growing the backing buffer requires `I::Item::default()` or
    /// `I::Item::clone()` and either operation panics.
    #[inline(always)]
    pub fn try_reserve_capacity(
        &mut self,
        capacity: usize,
    ) -> Result<(), TryReserveError> {
        self.buffer.try_reserve_capacity(capacity)
    }

    /// Advances the unread cursor without checking bounds.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of unread items to consume.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.unread_len()`.
    #[inline(always)]
    pub unsafe fn consume(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` fits the unread window.
        unsafe {
            self.buffer.consume(count);
        }
    }

    /// Copies unread items into an indexed output range without consuming them.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `output_index`: First destination index to write.
    /// - `count`: Number of unread items to copy.
    ///
    /// # Panics
    ///
    /// Panics if cloning an unread item panics. Debug builds also panic if the
    /// output range does not fit or `count` exceeds the unread item count.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the indexed destination range fits,
    /// `count <= self.unread_len()`, and the destination does not overlap the
    /// unread window.
    #[inline]
    pub unsafe fn copy_unread_to(
        &self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), output_index, count),
            "unchecked unread destination range exceeds output buffer"
        );
        debug_assert!(
            count <= self.buffer.available(),
            "unchecked unread copy exceeds available buffer items"
        );
        unsafe {
            let source = self.buffer.readable().get_unchecked(..count);
            let output =
                output.get_unchecked_mut(output_index..output_index + count);
            output.clone_from_slice(source);
        }
    }

    /// Polls one refill while preserving unread items.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` after appending at least one item, `Ok(false)` at
    /// EOF, or [`Poll::Pending`] while the wrapped input is not ready.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when the buffer is full with no
    /// consumed prefix to reclaim, or an error reported by the wrapped input.
    pub fn poll_fill_more(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<bool>> {
        // SAFETY: `inner` remains pinned for the duration of this projection.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if this.buffer.available() == 0 {
            this.buffer.clear();
        } else if this.buffer.spare_capacity() == 0 {
            this.buffer.compact();
            if this.buffer.spare_capacity() == 0 {
                return Poll::Ready(Err(Error::new(
                    ErrorKind::InvalidInput,
                    "buffered input is full; consume buffered items before refilling",
                )));
            }
        }

        let result = {
            // SAFETY: The projection does not move `inner`.
            let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
            inner.poll_read(cx, this.buffer.spare_mut())
        };
        match result {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(false)),
            Poll::Ready(Ok(read)) => {
                // SAFETY: `poll_read` validated `read` against the spare tail.
                unsafe {
                    this.buffer.advance(read);
                }
                Poll::Ready(Ok(true))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Polls refills until `count` unread items are available or EOF occurs.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `count`: Minimum number of unread items to make available.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` when `count` items are available, `Ok(false)` at
    /// EOF, or [`Poll::Pending`] while the wrapped input is not ready.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the buffer
    /// capacity, or an error reported by the wrapped input.
    pub fn poll_fill_until(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        count: usize,
    ) -> Poll<io::Result<bool>> {
        if count > self.as_ref().get_ref().buffer.capacity() {
            return Poll::Ready(Err(Error::new(
                ErrorKind::InvalidInput,
                "requested available items exceed buffered input capacity",
            )));
        }
        let mut ready_operations = 0;
        while self.as_ref().get_ref().buffer.available() < count {
            match self.as_mut().poll_fill_more(cx) {
                Poll::Ready(Ok(true)) => {
                    ready_operations += 1;
                    if self.as_ref().get_ref().buffer.available() < count
                        && ready_operations >= MAX_READY_OPERATIONS_PER_POLL
                    {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Ok(false)) => return Poll::Ready(Ok(false)),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(true))
    }

    /// Polls refills until `count` unread items are available.
    ///
    /// # Parameters
    ///
    /// - `cx`: Task context used to register a wake-up.
    /// - `count`: Minimum number of unread items to make available.
    ///
    /// # Returns
    ///
    /// Returns a ready success after `count` items are available, or
    /// [`Poll::Pending`] while the wrapped input is not ready.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] after discarding an incomplete
    /// unread window, [`ErrorKind::InvalidInput`] when `count` exceeds the
    /// buffer capacity, or an error reported by the wrapped input.
    pub fn poll_ensure_available(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        count: usize,
    ) -> Poll<io::Result<()>> {
        match self.as_mut().poll_fill_until(cx, count) {
            Poll::Ready(Ok(true)) => Poll::Ready(Ok(())),
            Poll::Ready(Ok(false)) => {
                // SAFETY: The complete unread window is available for discard.
                let this = unsafe { self.as_mut().get_unchecked_mut() };
                let available = this.buffer.available();
                unsafe {
                    this.buffer.consume(available);
                }
                Poll::Ready(Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                )))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<I> AsyncBufferedInput<I>
where
    I: AsyncInput + Unpin,
    I::Item: Clone + Default + Unpin,
{
    /// Asynchronously reads and appends at least one item to the unread buffer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` when data was appended or `Ok(false)` at end of
    /// input.
    ///
    /// # Errors
    ///
    /// Returns an error when the buffer is full and cannot reclaim space, or
    /// when the wrapped input returns an error.
    pub async fn fill_more_async(&mut self) -> io::Result<bool> {
        poll_fn(|cx| Pin::new(&mut *self).poll_fill_more(cx)).await
    }

    /// Asynchronously fills the unread buffer until it contains at least
    /// `count` items.
    ///
    /// # Parameters
    ///
    /// - `count`: Minimum number of unread items to make available.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` when `count` items are available or `Ok(false)` when
    /// the wrapped input reaches end of input first.
    ///
    /// # Errors
    ///
    /// Returns an error when `count` exceeds the buffer capacity or when the
    /// wrapped input returns an error.
    pub async fn fill_until_async(&mut self, count: usize) -> io::Result<bool> {
        poll_fn(|cx| Pin::new(&mut *self).poll_fill_until(cx, count)).await
    }

    /// Asynchronously ensures that the unread buffer contains at least `count`
    /// items.
    ///
    /// # Parameters
    ///
    /// - `count`: Minimum number of unread items to make available.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after `count` items are available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] after discarding incomplete unread
    /// data when the wrapped input ends early, [`ErrorKind::InvalidInput`] when
    /// `count` exceeds the buffer capacity, or an error from the wrapped input.
    pub async fn ensure_available_async(
        &mut self,
        count: usize,
    ) -> io::Result<()> {
        poll_fn(|cx| Pin::new(&mut *self).poll_ensure_available(cx, count))
            .await
    }
}

impl<I> AsyncInput for AsyncBufferedInput<I>
where
    I: AsyncInput,
    I::Item: Clone + Default,
{
    /// Item type produced by the wrapped input.
    type Item = I::Item;

    /// Reports that this input already buffers items.
    ///
    /// # Returns
    ///
    /// Always returns `true`.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Polls one read through the retained item buffer.
    ///
    /// A zero-length request completes immediately. Unread items are returned
    /// before polling the wrapped input.
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
    /// Returns [`Poll::Pending`] when the wrapped input is not ready. A ready
    /// result contains the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an I/O error reported by the wrapped input.
    ///
    /// # Panics
    ///
    /// May panic if a nonzero requested output range does not fit when the
    /// method copies buffered or newly fetched items into `output`.
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
        if count == 0 {
            return Poll::Ready(Ok(0));
        }

        // SAFETY: `inner` is never moved after projecting from this pinned
        // wrapper. `buffer` does not structurally pin any value.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if !this.buffer.is_empty() {
            let read = count.min(this.buffer.available());
            output[index..index + read]
                .clone_from_slice(&this.buffer.readable()[..read]);
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
                    .clone_from_slice(&this.buffer.readable()[..read]);
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
