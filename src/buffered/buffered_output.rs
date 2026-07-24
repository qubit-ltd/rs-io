// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Error,
    ErrorKind,
    Result,
    SeekFrom,
};
use std::mem::ManuallyDrop;
use std::ptr;

use crate::buffered::{
    DEFAULT_BUFFER_CAPACITY,
    EnsuredBufferedOutput,
};
use crate::traits::validate_write_count;
use crate::util::UncheckedSlice;
use crate::{
    Buffer,
    IntoInnerError,
    Output,
    Seekable,
    SeekableOutput,
};

/// Buffered item output over a wrapped output sink.
///
/// This type keeps a fixed-size item buffer in front of an underlying output so
/// small item writes can be accumulated before they are written to the I/O
/// target. Large writes may bypass the buffer after pending buffered items
/// have been flushed.
///
/// `BufferedOutput` is deliberately item-oriented. It performs no binary
/// encoding, text encoding, or record framing. Higher-level writers can either
/// use the [`Output`] implementation or write directly into
/// [`Self::spare_raw_parts_mut`] and then call [`Self::advance`] after
/// validating the range they initialized.
/// Call [`Self::flush`] before [`Self::into_parts`] when pending items must be
/// written before recovering the wrapped writer.
/// Dropping a `BufferedOutput` makes a best-effort attempt to write pending
/// buffered items, but drop-time errors are ignored. For arbitrary item types,
/// `BufferedOutput` also supports [`Seekable`]-based seeking in item offsets.
#[derive(Debug)]
pub struct BufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    inner: O,
    buffer: Buffer<O::Item>,
    panicked: bool,
}

impl<O> BufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    /// Creates a buffered item output with the default capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The output that receives items when the internal buffer is
    ///   flushed.
    ///
    /// # Returns
    ///
    /// A new buffered item output using `DEFAULT_BUFFER_CAPACITY`.
    #[inline(always)]
    #[must_use]
    pub fn new(inner: O) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered item output with at least the requested capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The output that receives items when the internal buffer is
    ///   flushed.
    /// * `capacity` - The requested internal buffer capacity in items.
    ///
    /// # Returns
    ///
    /// A new buffered item output whose actual buffer capacity is
    /// `capacity.max(1)`.
    #[inline(always)]
    #[must_use]
    pub fn with_capacity(inner: O, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
            panicked: false,
        }
    }

    /// Ensures that an output is buffered.
    ///
    /// # Parameters
    ///
    /// * `output` - The output to keep or wrap.
    ///
    /// # Returns
    ///
    /// [`EnsuredBufferedOutput::AlreadyBuffered`] when `output` already
    /// reports buffered status, or [`EnsuredBufferedOutput::Buffered`]
    /// wrapping `output` in [`BufferedOutput`] otherwise.
    #[inline(always)]
    #[must_use]
    pub fn ensure(output: O) -> EnsuredBufferedOutput<O> {
        if output.is_buffered() {
            EnsuredBufferedOutput::AlreadyBuffered(output)
        } else {
            EnsuredBufferedOutput::Buffered(Self::new(output))
        }
    }

    /// Ensures that an output is buffered and boxes the resulting output.
    ///
    /// # Parameters
    ///
    /// * `output` - The concrete output to keep or wrap.
    ///
    /// # Returns
    ///
    /// A boxed output trait object. The original output is boxed directly when
    /// it already reports buffered status; otherwise it is first wrapped in
    /// [`BufferedOutput`].
    #[inline(always)]
    #[must_use]
    pub fn ensure_boxed<'a>(output: O) -> Box<dyn Output<Item = O::Item> + 'a>
    where
        O: 'a,
        O::Item: 'a,
    {
        if output.is_buffered() {
            Box::new(output)
        } else {
            Box::new(Self::new(output))
        }
    }

    /// Returns a shared reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// An immutable reference to the underlying writer. Pending items may
    /// still be present in the internal buffer and are not flushed by this
    /// method.
    #[inline(always)]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped writer.
    ///
    /// Pending items may still be present in the internal buffer and are not
    /// flushed by this method. Writing through the returned reference can place
    /// new items before those pending items.
    ///
    /// # Returns
    ///
    /// A mutable reference to the underlying writer.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Flushes this buffered output and returns the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output after every buffered item and the wrapped
    /// output itself have been flushed.
    ///
    /// # Errors
    ///
    /// Returns an [`IntoInnerError`] carrying the flush error and this
    /// buffered output, preserving any unwritten suffix for a later retry.
    #[inline]
    pub fn into_inner(
        mut self,
    ) -> std::result::Result<O, IntoInnerError<Self>> {
        if let Err(error) = self.flush() {
            return Err(IntoInnerError::new(error, self));
        }
        let (inner, _) = self.into_parts();
        Ok(inner)
    }

    /// Compatibility alias for [`Self::into_inner`].
    ///
    /// # Returns
    ///
    /// Returns the wrapped output after every buffered item and the wrapped
    /// output itself have been flushed.
    ///
    /// # Errors
    ///
    /// Returns an [`IntoInnerError`] carrying the flush error and this
    /// buffered output, preserving any unwritten suffix for a later retry.
    #[inline(always)]
    pub fn try_into_inner(
        self,
    ) -> std::result::Result<O, IntoInnerError<Self>> {
        self.into_inner()
    }

    /// Consumes this buffered output without flushing pending items.
    ///
    /// This method performs no I/O. Pending items that have been accepted into
    /// the internal buffer but not written to the wrapped writer remain in the
    /// readable window of the returned buffer.
    ///
    /// # Returns
    ///
    /// The wrapped writer and the buffer holding pending items in logical write
    /// order.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (O, Buffer<O::Item>) {
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` will not be dropped, so reading both fields moves them
        // out exactly once. The `panicked` flag is intentionally discarded.
        unsafe {
            let inner = ptr::read(&this.inner);
            let buffer = ptr::read(&this.buffer);
            (inner, buffer)
        }
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
    /// Pending items are preserved and this method performs no I/O.
    ///
    /// # Errors
    ///
    /// Returns the original allocation error when the buffer cannot grow.
    #[inline]
    pub fn try_reserve_capacity(
        &mut self,
        capacity: usize,
    ) -> std::result::Result<(), std::collections::TryReserveError> {
        self.buffer.try_reserve_capacity(capacity)
    }

    /// Returns the unused capacity in the internal buffer.
    ///
    /// # Returns
    ///
    /// The number of items that can still be appended to the internal buffer
    /// before it must be flushed.
    #[inline(always)]
    #[must_use]
    pub fn spare_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Returns raw spare-buffer parts for hot-path callers.
    ///
    /// The returned slice is the full internal backing storage. `index` is the
    /// start of the spare item window, and `count` is the number of spare
    /// items. Callers that need a slice can use `&mut buffer[index..index +
    /// count]`; callers that already validated bounds can pass `buffer` and
    /// `index` directly to indexed unchecked codecs.
    ///
    /// Mutating items outside `index..index + count` changes pending output
    /// items and may corrupt the logical stream.
    ///
    /// # Returns
    ///
    /// The backing storage, the spare start index, and the spare item count.
    #[inline(always)]
    #[must_use]
    pub fn spare_raw_parts_mut(&mut self) -> (&mut [O::Item], usize, usize) {
        self.buffer.spare_raw_parts_mut()
    }

    /// Marks spare items as written without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of initialized spare items to make pending for
    ///   output.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.spare_capacity()` and
    /// that the corresponding items in the spare range reported by
    /// [`Self::spare_raw_parts_mut`] have been initialized.
    #[inline(always)]
    pub unsafe fn advance(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` is within spare capacity.
        unsafe {
            self.buffer.advance(count);
        }
    }

    /// Ensures that at least `count` items are available in the spare buffer.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of spare items required.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// items. Returns [`ErrorKind::InvalidInput`] if `count` exceeds the buffer
    /// capacity. Returns [`ErrorKind::InvalidData`] if the wrapped writer
    /// reports more items than the pending buffer range contained.
    pub fn ensure_spare_capacity(&mut self, count: usize) -> Result<()> {
        if count > self.buffer.capacity() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "requested spare capacity exceeds buffered output capacity",
            ));
        }
        if self.spare_capacity() < count {
            self.flush_buffer()?;
        }
        Ok(())
    }

    /// Writes items from the input slice and reports the accepted item count.
    ///
    /// This is the buffered implementation for single-write callers.
    /// Small inputs are appended to the buffer and reported as fully accepted;
    /// large inputs may be delegated to the wrapped writer after pending items
    /// are flushed.
    ///
    /// # Parameters
    ///
    /// * `input` - The items to write.
    ///
    /// # Returns
    ///
    /// The number of items accepted. Buffered writes return `input.len()`;
    /// direct writes return the item count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero items were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more items than the requested range contained.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    #[inline]
    pub unsafe fn write_unchecked(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<usize> {
        // Keep this boundary in sync with `std::io::BufWriter`: it uses
        // `< spare_capacity()` intentionally so buffer-sized writes skip the
        // memcpy+advance hot path. That path is only for strictly smaller
        // inputs as an in-memory append optimization.
        if count < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer(input, input_index, count);
            }
            Ok(count)
        } else {
            // SAFETY: The caller guarantees the source range is valid.
            unsafe { self.write_cold(input, input_index, count) }
        }
    }

    /// Writes items from the full input slice.
    ///
    /// # Parameters
    ///
    /// * `input` - Source items.
    ///
    /// # Returns
    ///
    /// The number of items accepted from `input`.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer. Returns
    /// [`ErrorKind::InvalidData`] if the wrapped writer reports accepting more
    /// items than requested.
    #[inline(always)]
    pub fn write(&mut self, input: &[O::Item]) -> Result<usize> {
        // SAFETY: The full input slice is a valid source range.
        let written = unsafe { self.write_unchecked(input, 0, input.len()) }?;
        validate_write_count(written, input.len())?;
        Ok(written)
    }

    /// Writes all items through the internal buffer.
    ///
    /// Small inputs are appended to the internal buffer.  Inputs that do not
    /// fit may flush the buffer first, and inputs at least as large as the
    /// buffer may be written directly to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - The items to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all items from `input` have been accepted.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero items were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more items than the requested range contained.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    #[inline]
    pub unsafe fn write_fully_unchecked(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<()> {
        // Keep this boundary in sync with `std::io::BufWriter`: it uses
        // `< spare_capacity()` intentionally so buffer-sized writes skip the
        // memcpy+advance hot path. That path is only for strictly smaller
        // inputs as an in-memory append optimization.
        if count < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer(input, input_index, count);
            }
            Ok(())
        } else {
            // SAFETY: The caller guarantees the source range is valid.
            unsafe { self.write_fully_cold(input, input_index, count) }
        }
    }

    /// Writes all items from the full input slice through the internal buffer.
    ///
    /// # Parameters
    ///
    /// * `input` - Source items.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer.
    #[inline(always)]
    pub fn write_fully(&mut self, input: &[O::Item]) -> Result<()> {
        // SAFETY: The full input slice is a valid source range.
        unsafe { self.write_fully_unchecked(input, 0, input.len()) }
    }

    /// Flushes buffered items and then flushes the wrapped output.
    ///
    /// # Returns
    ///
    /// `Ok(())` once pending buffered items and the wrapped output are
    /// flushed.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// items, [`ErrorKind::WriteZero`] if the wrapped writer cannot make
    /// progress while draining the buffer, [`ErrorKind::InvalidData`] if the
    /// writer reports an impossible item count, or any error returned by
    /// [`Output::flush`] on the wrapped output.
    #[inline(always)]
    pub fn flush(&mut self) -> Result<()> {
        self.flush_buffer()
            .and_then(|()| Output::flush(&mut self.inner))
    }

    /// Flushes buffered items to the wrapped writer.
    ///
    /// The method retries interrupted writes.  If an error occurs after some
    /// items have been written, the already-written items are removed from the
    /// front of the buffer and the unwritten suffix is kept for a later retry.
    ///
    /// # Returns
    ///
    /// `Ok(())` once all currently buffered items have been written to the
    /// wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped writer.
    /// Returns [`ErrorKind::WriteZero`] if the writer reports a zero-length
    /// write before all buffered items are drained. Returns
    /// [`ErrorKind::InvalidData`] if the writer reports more items than the
    /// pending buffer range contained.
    fn flush_buffer(&mut self) -> Result<()> {
        while !self.buffer.is_empty() {
            let position = self.buffer.position();
            let available = self.buffer.available();
            // SAFETY: `position..position + available` is the current readable
            // range maintained by `Buffer`.
            self.panicked = true;
            let result = unsafe {
                self.inner.write_unchecked(
                    self.buffer.data(),
                    position,
                    available,
                )
            };
            self.panicked = false;
            match result {
                Ok(0) => {
                    self.buffer.compact();
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write buffered data",
                    ));
                }
                Ok(written) => {
                    if let Err(error) = validate_write_count(written, available)
                    {
                        self.buffer.compact();
                        return Err(error);
                    }
                    // SAFETY: The validated count is in `0..=available`.
                    unsafe {
                        self.buffer.consume(written);
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => {
                    self.buffer.compact();
                    return Err(error);
                }
            }
        }
        self.buffer.clear();
        Ok(())
    }

    /// Returns the logical output position without flushing pending items.
    ///
    /// The returned position is the wrapped output's current position plus the
    /// number of items currently pending in this buffer.
    ///
    /// # Returns
    ///
    /// The logical stream position in output items.
    ///
    /// # Errors
    ///
    /// Returns any error produced while querying the wrapped output position.
    /// Returns [`ErrorKind::InvalidData`] if adding the pending item count
    /// overflows `u64`.
    pub fn stream_position(&mut self) -> Result<u64>
    where
        O: SeekableOutput,
    {
        let position =
            Seekable::seek_to(&mut self.inner, SeekFrom::Current(0))?;
        position
            .checked_add(self.buffer.available() as u64)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    "buffered pending items overflow wrapped output position",
                )
            })
    }

    /// Seeks the wrapped output in items, flushing buffered items first.
    ///
    /// Pending items accepted into the internal buffer are written to the
    /// wrapped output before [`Seekable::seek_to`] is invoked, so the seek
    /// position is relative to data already committed to the underlying sink.
    ///
    /// # Parameters
    ///
    /// * `position` - The target seek position in output items.
    ///
    /// # Returns
    ///
    /// The new stream position in items.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// items, [`ErrorKind::WriteZero`] if the wrapped output cannot make
    /// progress while draining the buffer, [`ErrorKind::InvalidData`] if the
    /// writer reports an impossible item count, or any error returned by
    /// [`Seekable::seek_to`] on the wrapped output.
    #[inline(always)]
    pub fn seek_to(&mut self, position: SeekFrom) -> Result<u64>
    where
        O: SeekableOutput,
    {
        match position {
            SeekFrom::Current(0) => self.stream_position(),
            other => self
                .flush_buffer()
                .and_then(|()| Seekable::seek_to(&mut self.inner, other)),
        }
    }

    /// Writes items into the internal buffer without checking spare capacity.
    ///
    /// # Parameters
    ///
    /// * `input` - The source items.
    /// * `input_index` - The starting index in `input`.
    /// * `count` - The number of items to copy.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `input_index..input_index + count` is valid
    /// in `input`, that `count <= self.spare_capacity()`, and that the copied
    /// source range does not overlap with the destination range in the internal
    /// buffer.
    #[inline(always)]
    unsafe fn write_to_buffer(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), input_index, count),
            "unchecked write range exceeds input buffer"
        );
        debug_assert!(
            count <= self.spare_capacity(),
            "unchecked write exceeds spare buffer capacity"
        );
        let (destination, destination_index, _) =
            self.buffer.spare_raw_parts_mut();
        // SAFETY: The caller guarantees valid source and destination ranges and
        // that they do not overlap.
        unsafe {
            UncheckedSlice::copy_nonoverlapping(
                input,
                input_index,
                destination,
                destination_index,
                count,
            );
            self.buffer.advance(count);
        }
    }

    /// Writes items to the wrapped writer and validates the reported count.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `input_index` - Start index inside `input`.
    /// * `count` - Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// The number of items accepted by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns the wrapped writer's I/O error, or [`ErrorKind::InvalidData`]
    /// if it reports an item count larger than `count`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    #[inline(always)]
    unsafe fn write_inner(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: The caller guarantees the source range is valid.
        let written =
            unsafe { self.inner.write_unchecked(input, input_index, count) }?;
        validate_write_count(written, count)?;
        Ok(written)
    }

    /// Writes all items in an indexed source range to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `input_index` - Start index inside `input`.
    /// * `count` - Number of items to write.
    ///
    /// # Errors
    ///
    /// Returns the wrapped writer's I/O error, [`ErrorKind::WriteZero`] if the
    /// writer cannot make progress, or [`ErrorKind::InvalidData`] if it
    /// reports an impossible item count.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    unsafe fn write_fully_inner(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<()> {
        let mut written = 0;
        while written < count {
            let remaining = count - written;
            // SAFETY: `written < count`, so this suffix remains inside the
            // caller-validated source range.
            match unsafe {
                self.write_inner(input, input_index + written, remaining)
            } {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Handles slow-path raw writes that must flush or bypass the buffer.
    ///
    /// # Parameters
    ///
    /// * `input` - The items to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all items from `input` have been accepted either by the
    /// buffer or by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero items were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more items than the requested range contained.
    #[cold]
    #[inline(never)]
    unsafe fn write_fully_cold(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<()> {
        if count > self.spare_capacity() {
            self.flush_buffer()?;
        }
        if count >= self.buffer.capacity() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.write_fully_inner(input, input_index, count) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer(input, input_index, count);
            }
            Ok(())
        }
    }

    /// Handles slow-path raw writes for single-write semantics.
    ///
    /// The method may accept fewer items than the input length when the write
    /// is delegated directly to the wrapped output.
    ///
    /// # Parameters
    ///
    /// * `input` - The items to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// The number of items accepted. Buffered writes return `input.len()`;
    /// direct writes return the item count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending items or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero items were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more items than the requested range contained.
    #[cold]
    #[inline(never)]
    unsafe fn write_cold(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<usize> {
        if count > self.spare_capacity() {
            self.flush_buffer()?;
        }
        if count >= self.buffer.capacity() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.write_inner(input, input_index, count) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer(input, input_index, count);
            }
            Ok(count)
        }
    }
}

impl<O> Output for BufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    type Item = O::Item;

    /// Reports that this output already buffers items internally.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Writes items through the internal buffer.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe {
            BufferedOutput::write_unchecked(self, input, input_index, count)
        }
    }

    /// Writes items from the full input slice.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        BufferedOutput::write(self, input)
    }

    /// Writes all items through the internal buffer.
    #[inline(always)]
    unsafe fn write_fully_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<()> {
        // SAFETY: Forwarded from the trait caller.
        unsafe {
            BufferedOutput::write_fully_unchecked(self, input, index, count)
        }
    }

    /// Writes all items from the full input slice through the internal buffer.
    #[inline(always)]
    fn write_fully(&mut self, input: &[Self::Item]) -> Result<()> {
        BufferedOutput::write_fully(self, input)
    }

    /// Flushes pending items through the internal buffer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        BufferedOutput::flush(self)
    }
}

impl<O> Seekable for BufferedOutput<O>
where
    O: SeekableOutput,
    <O as Output>::Item: Copy + Default,
{
    type Unit = <O as Output>::Item;

    /// Seeks the buffered output in item offsets.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        BufferedOutput::seek_to(self, position)
    }
}

impl<O> Drop for BufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    fn drop(&mut self) {
        if !self.panicked {
            drop(self.flush_buffer());
        }
    }
}
