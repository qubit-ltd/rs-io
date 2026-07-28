// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    collections::TryReserveError,
    io::{
        Error,
        ErrorKind,
        Result,
        SeekFrom,
    },
};

use crate::buffered::{
    DEFAULT_BUFFER_CAPACITY,
    EnsuredBufferedInput,
};
use crate::traits::validate_read_count;
use crate::util::UncheckedSlice;
use crate::{
    Buffer,
    Input,
    Seekable,
    SeekableInput,
};

/// Buffered item input over a wrapped input source.
///
/// This type owns a wrapped input object and an internal item buffer. It keeps
/// unread items in `buffer[position..limit]` so callers can inspect or consume
/// the current item window before refilling it.
///
/// `BufferedInput` is deliberately item-oriented. It performs no binary
/// decoding, text decoding, or record parsing; higher-level stream adapters can
/// build those concerns on top of [`Self::ensure_available`],
/// [`Self::copy_unread_to`], and [`Self::read_unchecked`]. The type also
/// implements [`Input`] and [`Seekable`] directly, so it can be passed to
/// item-oriented APIs without converting through standard byte traits.
///
/// # Type Parameters
///
/// - `I`: Wrapped item input type.
#[must_use]
#[derive(Debug)]
pub struct BufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// Input supplying items to the buffer.
    inner: I,
    /// Storage retaining fetched but unread items.
    buffer: Buffer<I::Item>,
}

/// Appends one chunk from a type-erased input to a buffer.
///
/// Interrupted reads are retried until another outcome is observed.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies the next chunk.
/// - `buffer`: Buffer whose spare tail receives the chunk.
///
/// # Returns
///
/// Returns `Ok(true)` when at least one item is appended, or `Ok(false)` at
/// EOF.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if the input reports more items than the
/// spare tail can hold. Other non-interrupted errors are propagated from the
/// input.
///
/// # Panics
///
/// Panics in debug builds if `buffer` has no spare capacity.
fn read_more_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
) -> Result<bool>
where
    T: Copy + Default,
{
    let count = buffer.spare_capacity();
    debug_assert!(count > 0, "buffer has no tail capacity");
    loop {
        let limit = buffer.limit();
        // SAFETY: `limit` is within `buffer`, and `count` is the remaining
        // capacity from `limit` to the end of `buffer`.
        match unsafe { inner.read_unchecked(buffer.data_mut(), limit, count) } {
            Ok(0) => return Ok(false),
            Ok(read) => {
                validate_read_count(read, count)?;
                // SAFETY: The validated count fits the spare buffer range.
                unsafe {
                    buffer.advance(read);
                }
                return Ok(true);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Preserves unread items and appends one chunk from a type-erased input.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies the next chunk.
/// - `buffer`: Buffer whose unread items must be preserved.
///
/// # Returns
///
/// Returns `Ok(true)` when at least one item is appended, or `Ok(false)` at
/// EOF.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when the buffer is full and has no
/// consumed prefix to reclaim. Returns [`ErrorKind::InvalidData`] for an
/// invalid count reported by `inner`; other non-interrupted errors are
/// propagated.
fn fill_more_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
) -> Result<bool>
where
    T: Copy + Default,
{
    if buffer.available() == 0 {
        buffer.clear();
    } else if buffer.spare_capacity() == 0 {
        buffer.compact();
        if buffer.spare_capacity() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "buffered input is full; consume buffered items before refilling",
            ));
        }
    }
    read_more_impl(inner, buffer)
}

/// Refills a buffer to a requested unread item count.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies additional items.
/// - `buffer`: Buffer retaining unread items.
/// - `count`: Required unread item count.
///
/// # Returns
///
/// Returns `Ok(true)` when `count` items are available, or `Ok(false)` when EOF
/// is reached first.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the buffer
/// capacity. Returns [`ErrorKind::InvalidData`] for an invalid count reported
/// by `inner`; other non-interrupted input errors are propagated.
fn fill_until_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    count: usize,
) -> Result<bool>
where
    T: Copy + Default,
{
    if count > buffer.capacity() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "requested available items exceed buffered input capacity",
        ));
    }
    while buffer.available() < count {
        let available = buffer.available();
        if available == 0 {
            buffer.clear();
        } else {
            let missing = count - available;
            if buffer.spare_capacity() < missing {
                buffer.compact();
            }
        }
        if !read_more_impl(inner, buffer)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Ensures a requested unread item count through a type-erased input.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies additional items.
/// - `buffer`: Buffer retaining unread items.
/// - `count`: Required unread item count.
///
/// # Returns
///
/// Returns `Ok(())` when `count` unread items are available.
///
/// # Errors
///
/// Returns [`ErrorKind::UnexpectedEof`] when EOF is reached first,
/// [`ErrorKind::InvalidInput`] when `count` exceeds the buffer capacity, or
/// [`ErrorKind::InvalidData`] for an invalid count reported by `inner`. Other
/// non-interrupted input errors are propagated.
fn ensure_available_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    count: usize,
) -> Result<()>
where
    T: Copy + Default,
{
    if fill_until_impl(inner, buffer, count)? {
        return Ok(());
    }
    let available = buffer.available();
    // SAFETY: `available` is the current readable item count.
    unsafe {
        buffer.consume(available);
    }
    Err(Error::new(
        ErrorKind::UnexpectedEof,
        "failed to fill whole buffer",
    ))
}

/// Reads an indexed range through a type-erased input and retained buffer.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies items when the buffer is empty.
/// - `buffer`: Buffer retaining unread items.
/// - `output`: Destination item slice.
/// - `output_index`: Starting destination index.
/// - `count`: Maximum number of items to read.
///
/// # Returns
///
/// Returns the number of items copied or read.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if `inner` reports more than `count`
/// items. Other errors are propagated from `inner`.
///
/// # Panics
///
/// Panics in debug builds if the requested output range does not fit.
///
/// # Safety
///
/// The caller must guarantee that `output_index..output_index + count` is a
/// valid range inside `output` and that the addition does not overflow.
#[inline(always)]
unsafe fn read_unchecked_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize>
where
    T: Copy + Default,
{
    debug_assert!(
        UncheckedSlice::range_fits(output.len(), output_index, count),
        "unchecked read output range exceeds destination buffer"
    );
    if count == 0 {
        return Ok(0);
    }
    if buffer.available() == 0 {
        buffer.clear();
        if count >= buffer.capacity() {
            // SAFETY: Forwarded from the caller.
            let read =
                unsafe { inner.read_unchecked(output, output_index, count) }?;
            validate_read_count(read, count)?;
            return Ok(read);
        }
        if !read_more_impl(inner, buffer)? {
            return Ok(0);
        }
    }
    let read_count = count.min(buffer.available());
    // SAFETY: The count fits the output range and readable buffer window.
    unsafe {
        buffer.copy_to(output, output_index, read_count);
    }
    Ok(read_count)
}

/// Fills an indexed range through a type-erased input and retained buffer.
///
/// # Type Parameters
///
/// - `T`: Copyable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies items after buffered items are consumed.
/// - `buffer`: Buffer retaining unread items.
/// - `output`: Destination item slice.
/// - `output_index`: Starting destination index.
/// - `count`: Number of items requested.
///
/// # Returns
///
/// Returns the number of items read before the range filled or EOF was
/// reached.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if `inner` reports more items than
/// requested. Other non-interrupted errors are propagated from `inner`.
///
/// # Panics
///
/// Panics in debug builds if the requested output range does not fit.
///
/// # Safety
///
/// The caller must guarantee that `output_index..output_index + count` is a
/// valid range inside `output` and that the addition does not overflow.
unsafe fn read_fully_unchecked_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize>
where
    T: Copy + Default,
{
    debug_assert!(
        UncheckedSlice::range_fits(output.len(), output_index, count),
        "unchecked read-fully output range exceeds destination buffer"
    );
    if count == 0 {
        return Ok(0);
    }

    let available = buffer.available();
    if available >= count {
        // SAFETY: Enough unread items and destination space are available.
        unsafe {
            buffer.copy_to(output, output_index, count);
        }
        return Ok(count);
    }

    let mut total = 0;
    if available > 0 {
        // SAFETY: The available items fit the caller's destination range.
        unsafe {
            buffer.copy_to(output, output_index, available);
        }
        total = available;
    }

    let remaining = count - total;
    if remaining >= buffer.capacity() {
        buffer.clear();
        loop {
            // SAFETY: The remaining suffix is inside the caller's range.
            match unsafe {
                inner.read_fully_unchecked(
                    output,
                    output_index + total,
                    remaining,
                )
            } {
                Ok(read) => {
                    validate_read_count(read, remaining)?;
                    return Ok(total + read);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
    }

    while total < count {
        let remaining = count - total;
        // SAFETY: The remaining suffix is inside the caller's range.
        match unsafe {
            read_unchecked_impl(
                inner,
                buffer,
                output,
                output_index + total,
                remaining,
            )
        } {
            Ok(0) => break,
            Ok(read) => total += read,
            Err(error) => return Err(error),
        }
    }
    Ok(total)
}

impl<I> BufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// Creates a buffered item input with the default capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Input object wrapped by this buffer.
    ///
    /// # Returns
    ///
    /// Returns a new buffered item input whose internal buffer has at least
    /// `DEFAULT_BUFFER_CAPACITY` items.
    ///
    /// # Panics
    ///
    /// Panics if `I::Item::default()` panics or the default backing length
    /// exceeds [`Vec`]'s supported capacity.
    #[inline(always)]
    pub fn new(inner: I) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered item input with at least the requested capacity.
    ///
    /// The actual capacity is raised to `1` when the requested value is `0`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Input object wrapped by this buffer.
    /// - `capacity`: Requested internal buffer capacity, in items.
    ///
    /// # Returns
    ///
    /// Returns a new buffered item input whose internal buffer capacity is
    /// `capacity.max(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `I::Item::default()` panics or the requested backing length
    /// exceeds [`Vec`]'s supported capacity.
    #[inline]
    pub fn with_capacity(inner: I, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Tries to create a buffered item input with a requested capacity.
    ///
    /// # Parameters
    ///
    /// - `inner`: Input supplying items to the buffer.
    /// - `capacity`: Requested internal buffer capacity in items.
    ///
    /// # Returns
    ///
    /// Returns a buffered item input whose actual capacity is
    /// `capacity.max(1)`.
    ///
    /// # Errors
    ///
    /// Returns the allocation error when the backing buffer cannot be
    /// allocated.
    ///
    /// # Panics
    ///
    /// Panics if initializing the backing buffer requires
    /// `I::Item::default()` and it panics.
    #[inline]
    pub fn try_with_capacity(
        inner: I,
        capacity: usize,
    ) -> std::result::Result<Self, TryReserveError> {
        Ok(Self {
            inner,
            buffer: Buffer::try_with_capacity(capacity)?,
        })
    }

    /// Ensures that an input is buffered.
    ///
    /// # Parameters
    ///
    /// - `input`: Input to keep or wrap.
    ///
    /// # Returns
    ///
    /// [`EnsuredBufferedInput::AlreadyBuffered`] when `input` already reports
    /// buffered status, or [`EnsuredBufferedInput::Buffered`] wrapping `input`
    /// in [`BufferedInput`] otherwise.
    ///
    /// This check only observes [`Input::is_buffered`]. Standard-library
    /// [`std::io::BufReader`] values use the blanket [`std::io::Read`]
    /// implementation and are therefore not detected.
    ///
    /// # Panics
    ///
    /// Panics while wrapping an unbuffered input if `I::Item::default()` panics
    /// or the default backing length exceeds [`Vec`]'s supported capacity.
    #[inline]
    pub fn ensure(input: I) -> EnsuredBufferedInput<I> {
        if input.is_buffered() {
            EnsuredBufferedInput::AlreadyBuffered(input)
        } else {
            EnsuredBufferedInput::Buffered(Self::new(input))
        }
    }

    /// Ensures that an input is buffered and boxes the resulting input.
    ///
    /// # Type Parameters
    ///
    /// - `'a`: Lifetime required for the returned trait object.
    ///
    /// # Parameters
    ///
    /// - `input`: Concrete input to keep or wrap.
    ///
    /// # Returns
    ///
    /// A boxed input trait object. The original input is boxed directly when it
    /// already reports buffered status; otherwise it is first wrapped in
    /// [`BufferedInput`].
    ///
    /// This check only observes [`Input::is_buffered`]. Standard-library
    /// [`std::io::BufReader`] values use the blanket [`std::io::Read`]
    /// implementation and are therefore not detected.
    ///
    /// # Panics
    ///
    /// Panics while wrapping an unbuffered input if `I::Item::default()` panics
    /// or the default backing length exceeds [`Vec`]'s supported capacity.
    #[inline]
    #[must_use]
    pub fn ensure_boxed<'a>(input: I) -> Box<dyn Input<Item = I::Item> + 'a>
    where
        I: 'a,
        I::Item: 'a,
    {
        if input.is_buffered() {
            Box::new(input)
        } else {
            Box::new(Self::new(input))
        }
    }

    /// Returns a shared reference to the wrapped input object.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the inner input object.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped input object.
    ///
    /// Mutating the wrapped object directly may invalidate assumptions about
    /// items already buffered by this value.
    ///
    /// # Returns
    ///
    /// Returns an exclusive reference to the wrapped input object.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this buffered input and returns the wrapped input object plus
    /// its buffer.
    ///
    /// This method performs no I/O. Units that have already been read from the
    /// wrapped input but not consumed by this buffered input remain in the
    /// readable window of the returned buffer. To continue the same logical
    /// stream, consume that window before reading from the returned input,
    /// whose physical position may already be ahead of those items.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input object and the buffer holding unread items in
    /// logical read order.
    #[inline(always)]
    #[must_use = "the returned inner input and unread buffer must be handled"]
    pub fn into_parts(self) -> (I, Buffer<I::Item>) {
        (self.inner, self.buffer)
    }

    /// Returns the internal buffer capacity.
    ///
    /// # Returns
    ///
    /// The total number of items that can be held by the internal buffer.
    #[inline(always)]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Tries to ensure that the internal buffer has at least `capacity` items.
    ///
    /// Buffered unread items are preserved and this method performs no I/O.
    ///
    /// # Parameters
    ///
    /// - `capacity`: Required total buffer capacity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the requested capacity is available.
    ///
    /// # Errors
    ///
    /// Returns the original allocation error when the buffer cannot grow.
    ///
    /// # Panics
    ///
    /// Panics if growing the buffer requires `I::Item::default()` and it
    /// panics.
    #[inline(always)]
    pub fn try_reserve_capacity(
        &mut self,
        capacity: usize,
    ) -> std::result::Result<(), std::collections::TryReserveError> {
        self.buffer.try_reserve_capacity(capacity)
    }

    /// Returns the number of unread items currently buffered.
    ///
    /// # Returns
    ///
    /// Returns the length of `buffer[position..limit]`, in items.
    #[inline(always)]
    #[must_use]
    pub fn unread_len(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the currently buffered unread items.
    ///
    /// # Returns
    ///
    /// Returns the `buffer[position..limit]` unread item window. The slice may
    /// be empty when no items are currently buffered.
    #[inline(always)]
    #[must_use]
    pub fn unread(&self) -> &[I::Item] {
        self.buffer.readable()
    }

    /// Advances the unread cursor without checking bounds.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of currently unread items to consume.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `count > self.unread_len()`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.unread_len()`.
    #[inline(always)]
    pub unsafe fn consume(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` is within the readable
        // input window.
        unsafe {
            self.buffer.consume(count);
        }
    }

    /// Copies unread items into an indexed output range without consuming them.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination storage that receives a copy of unread items.
    /// - `output_index`: Start index inside `output`.
    /// - `count`: Number of unread items to copy.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the output range does not fit or `count`
    /// exceeds the unread item count.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output`, that the addition does not overflow, that
    /// `count <= self.unread_len()`, and that the destination range does not
    /// overlap with the unread range stored inside this buffer.
    #[inline]
    pub unsafe fn copy_unread_to(
        &self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), output_index, count),
            "unchecked unread copy output range exceeds destination buffer",
        );
        debug_assert!(
            count <= self.unread_len(),
            "unchecked unread copy exceeds available input buffer",
        );
        // SAFETY: The caller guarantees that the destination range is valid,
        // non-overlapping, and that `count` unread items are currently
        // available.
        unsafe {
            UncheckedSlice::copy_nonoverlapping(
                self.buffer.readable(),
                0,
                output,
                output_index,
                count,
            );
        }
    }

    /// Refills the internal buffer after preserving unread items.
    ///
    /// Consumed items may be discarded, and unread items may be moved to the
    /// front of the buffer before the wrapped reader is called.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if at least one item was appended, or `Ok(false)` at
    /// EOF.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    /// Returns [`ErrorKind::InvalidInput`] when the buffer is already full and
    /// no unread items have been consumed; callers must consume buffered items
    /// before refilling in that state.
    #[inline(always)]
    pub fn fill_more(&mut self) -> Result<bool> {
        fill_more_impl(&mut self.inner, &mut self.buffer)
    }

    /// Refills the buffer until at least `count` unread items are available.
    ///
    /// This method may discard consumed items or move unread items to the front
    /// of the buffer before reading more data. It stops as soon as the unread
    /// window reaches `count` items or the wrapped reader reaches EOF.
    ///
    /// # Parameters
    ///
    /// - `count`: Minimum number of unread items required.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if at least `count` unread items are buffered.
    /// `Ok(false)` means EOF was reached before the requested item count became
    /// available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the internal
    /// buffer capacity. Returns [`ErrorKind::InvalidData`] if the wrapped
    /// reader reports more items than the spare buffer range could hold.
    /// Returns any non-interrupted I/O error produced by the wrapped reader
    /// while refilling the buffer.
    #[inline(always)]
    pub fn fill_until(&mut self, count: usize) -> Result<bool> {
        fill_until_impl(&mut self.inner, &mut self.buffer, count)
    }

    /// Ensures that at least `count` unread items are available.
    ///
    /// Unlike [`Self::fill_until`], this method treats EOF before the requested
    /// item count as [`ErrorKind::UnexpectedEof`]. Any partial items buffered
    /// before EOF are consumed so callers observe the same logical position as
    /// a failed exact read.
    ///
    /// # Parameters
    ///
    /// - `count`: Minimum number of unread items required.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` once at least `count` unread items are available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before `count`
    /// items are available. Returns [`ErrorKind::InvalidInput`] when `count`
    /// exceeds the internal buffer capacity. Returns [`ErrorKind::InvalidData`]
    /// if the wrapped reader reports more items than the spare buffer range
    /// could hold. Returns any non-interrupted I/O error produced by the
    /// wrapped reader while refilling the buffer.
    #[inline(always)]
    pub fn ensure_available(&mut self, count: usize) -> Result<()> {
        ensure_available_impl(&mut self.inner, &mut self.buffer, count)
    }

    /// Reads items through the internal buffer into an indexed output range.
    ///
    /// If the internal buffer is empty and `count` is at least as large as the
    /// internal buffer capacity, the read is delegated directly to the wrapped
    /// reader to avoid an unnecessary copy. Otherwise, items are served from
    /// the internal buffer.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination storage that receives items.
    /// - `output_index`: Start index inside `output`.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items written into
    /// `output[output_index..output_index + count]`. A return value of `0`
    /// means that `count` was zero or EOF was reached before any items were
    /// read.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced by the wrapped reader. Returns
    /// [`ErrorKind::InvalidData`] if the wrapped reader reports more items
    /// than the requested destination range could hold. Interrupted reads are
    /// retried when the method refills the internal buffer through
    /// `read_more`; direct delegated reads follow the wrapped input's own
    /// single-read behavior.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output` and that the addition does not overflow.
    #[inline(always)]
    pub unsafe fn read_unchecked(
        &mut self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the caller.
        unsafe {
            read_unchecked_impl(
                &mut self.inner,
                &mut self.buffer,
                output,
                output_index,
                count,
            )
        }
    }

    /// Reads items into the full output slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination storage.
    ///
    /// # Returns
    ///
    /// Returns the number of items read into `output`.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation.
    #[inline(always)]
    pub fn read(&mut self, output: &mut [I::Item]) -> Result<usize> {
        // SAFETY: The caller ensured the destination slice is valid.
        unsafe { self.read_unchecked(output, 0, output.len()) }
    }

    /// Reads items through the internal buffer until the target range is full
    /// or EOF is reached.
    ///
    /// Buffered unread items are copied first. If the remaining destination
    /// range is at least as large as the internal buffer capacity, the
    /// remaining read is delegated directly to the wrapped input to avoid
    /// refilling and draining the internal buffer.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination storage that receives items.
    /// - `output_index`: Start index inside `output`.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items written into
    /// `output[output_index..output_index + count]`.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    /// Returns [`ErrorKind::InvalidData`] if the wrapped reader reports more
    /// items than requested.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is a
    /// valid range inside `output` and that the addition does not overflow.
    #[inline(always)]
    pub unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the caller.
        unsafe {
            read_fully_unchecked_impl(
                &mut self.inner,
                &mut self.buffer,
                output,
                output_index,
                count,
            )
        }
    }

    /// Reads items into the full output slice until it is full or EOF is
    /// reached.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination storage to fill as far as possible.
    ///
    /// # Returns
    ///
    /// Returns the number of items written into `output`.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    #[inline(always)]
    pub fn read_fully(&mut self, output: &mut [I::Item]) -> Result<usize> {
        // SAFETY: The full output slice is a valid destination range.
        unsafe { self.read_fully_unchecked(output, 0, output.len()) }
    }

    /// Seeks the wrapped reader and discards buffered items after success.
    ///
    /// For [`SeekFrom::Current`], the offset is adjusted by the number of
    /// unread items already buffered, so seeking is relative to the logical
    /// position observed by callers of this buffered input.
    ///
    /// # Parameters
    ///
    /// - `position`: Target seek position.
    ///
    /// # Returns
    ///
    /// Returns the new absolute stream position reported by the wrapped reader,
    /// in items.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if a [`SeekFrom::Current`] offset
    /// cannot be adjusted by the unread buffered item count. Returns any seek
    /// error produced by the wrapped reader.
    pub fn seek_to(&mut self, position: SeekFrom) -> Result<u64>
    where
        I: SeekableInput,
    {
        match position {
            SeekFrom::Current(offset) => {
                if self.seek_within_buffer(offset) {
                    return self.stream_position();
                }
                let position = self.seek_relative_slow(offset)?;
                self.discard_buffer();
                Ok(position)
            }
            other => {
                // Absolute seeks currently delegate to the wrapped input and
                // discard the buffer. A future optimization could satisfy
                // SeekFrom::Start targets that fall inside the retained
                // backing buffer by moving only the buffer cursor, provided we
                // track or cheaply derive the absolute position of the
                // retained window.
                let position = Seekable::seek_to(&mut self.inner, other)?;
                self.discard_buffer();
                Ok(position)
            }
        }
    }

    /// Returns the logical input position without discarding buffered items.
    ///
    /// The returned position is the wrapped input's current position minus the
    /// number of items currently unread in this buffer.
    ///
    /// # Returns
    ///
    /// Returns the logical stream position in input items.
    ///
    /// # Errors
    ///
    /// Returns any seek error produced while querying the wrapped input's
    /// current position. Returns [`ErrorKind::InvalidData`] if the wrapped
    /// input reports a position before the unread buffered window.
    #[inline]
    pub fn stream_position(&mut self) -> Result<u64>
    where
        I: SeekableInput,
    {
        let position =
            Seekable::seek_to(&mut self.inner, SeekFrom::Current(0))?;
        let unread = self.unread_len() as u64;
        position.checked_sub(unread).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "buffered unread items exceed wrapped input position",
            )
        })
    }

    /// Seeks the wrapped input relative to the logical current position.
    ///
    /// # Parameters
    ///
    /// - `offset`: Relative offset in input items.
    ///
    /// # Returns
    ///
    /// Returns the new position reported by the wrapped input.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if `offset` cannot be adjusted by
    /// the unread buffered item count. Returns any seek error produced by
    /// the wrapped input.
    #[inline]
    fn seek_relative_slow(&mut self, offset: i64) -> Result<u64>
    where
        I: SeekableInput,
    {
        let unread = i64::try_from(self.unread_len()).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "buffered unread item count exceeds i64",
            )
        })?;
        let adjusted = offset.checked_sub(unread).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "current seek offset underflows after buffered adjustment",
            )
        })?;
        Seekable::seek_to(&mut self.inner, SeekFrom::Current(adjusted))
    }

    /// Attempts to satisfy a relative seek inside the current buffer window.
    ///
    /// Positive offsets consume unread items. Negative offsets can rewind into
    /// the still-retained consumed prefix of the backing buffer. If the target
    /// is outside the retained buffer contents, the caller must seek the
    /// wrapped input instead.
    ///
    /// # Parameters
    ///
    /// - `offset`: Relative offset in input items.
    ///
    /// # Returns
    ///
    /// Returns `true` when the buffer cursor was moved. Returns `false` when
    /// the caller must delegate the seek to the wrapped input.
    #[must_use]
    fn seek_within_buffer(&mut self, offset: i64) -> bool {
        if offset >= 0 {
            let count = offset as u64;
            if count <= self.unread_len() as u64 {
                let count = count as usize;
                // SAFETY: The branch proves that `count` is within the unread
                // buffer window.
                unsafe {
                    self.buffer.consume(count);
                }
                return true;
            }
            return false;
        }
        let count = offset.unsigned_abs();
        if count <= self.buffer.position() as u64 {
            let count = count as usize;
            // SAFETY: The branch proves that `count` is within the retained
            // consumed prefix.
            unsafe {
                self.buffer.rewind(count);
            }
            return true;
        }
        false
    }

    /// Invalidates all buffered items.
    ///
    /// After this call, the buffer is considered empty and subsequent reads
    /// will refill it from the wrapped input.
    #[inline(always)]
    fn discard_buffer(&mut self) {
        self.buffer.clear();
    }
}

impl<I> Input for BufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// Item type produced by the buffered input.
    type Item = I::Item;

    /// Reports that this input already buffers items internally.
    ///
    /// # Returns
    ///
    /// Always returns `true`.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Reads items through the internal buffer.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `output_index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidData`] if the wrapped input reports an
    /// invalid count. Other errors are propagated from the wrapped input.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The range `output_index..output_index + count` must be valid for
    /// `output`.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe {
            BufferedInput::read_unchecked(self, output, output_index, count)
        }
    }

    /// Reads items into the full output slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidData`] if the wrapped input reports an
    /// invalid count. Other errors are propagated from the wrapped input.
    #[inline(always)]
    fn read(&mut self, output: &mut [I::Item]) -> Result<usize> {
        BufferedInput::read(self, output)
    }

    /// Reads items through the internal buffer until the target range is full
    /// or EOF is reached.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Number of items requested.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the range filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidData`] if the wrapped input reports an
    /// invalid count. Other non-interrupted errors are propagated from the
    /// wrapped input.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
    #[inline(always)]
    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe {
            BufferedInput::read_fully_unchecked(self, output, index, count)
        }
    }

    /// Reads items into the full output slice through the internal buffer.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the slice filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidData`] if the wrapped input reports an
    /// invalid count. Other non-interrupted errors are propagated from the
    /// wrapped input.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        BufferedInput::read_fully(self, output)
    }
}

impl<I> Seekable for BufferedInput<I>
where
    I: SeekableInput,
    <I as Input>::Item: Copy + Default,
{
    /// Item unit used for seek offsets.
    type Unit = <I as Input>::Item;

    /// Seeks the buffered input in item offsets.
    ///
    /// # Parameters
    ///
    /// - `position`: Target item offset.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute item position.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if a relative offset cannot be
    /// adjusted by the unread item count. Returns [`ErrorKind::InvalidData`] if
    /// the wrapped position precedes the unread window. Other seek errors are
    /// propagated from the wrapped input.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        BufferedInput::seek_to(self, position)
    }
}
