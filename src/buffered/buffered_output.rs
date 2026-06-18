// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Result, Seek, SeekFrom, Write};
use std::mem::ManuallyDrop;
use std::ptr;

use crate::buffered::DEFAULT_BUFFER_CAPACITY;
use crate::util::UncheckedSlice;
use crate::{Buffer, Output, Seekable, SeekableOutput};

/// Buffered unit output over a wrapped output sink.
///
/// This type keeps a fixed-size unit buffer in front of an underlying output so
/// small unit writes can be accumulated before they are written to the I/O
/// target. Large writes may bypass the buffer after pending buffered units
/// have been flushed.
///
/// `BufferedOutput` is deliberately unit-oriented. It performs no binary
/// encoding, text encoding, or record framing. Higher-level writers can either
/// use the standard [`Write`] implementation or write directly into
/// [`Self::spare_raw_parts_mut`] and then call [`Self::advance`] after
/// validating the range they initialized.
/// Callers that need to recover the wrapped writer should call
/// [`Write::flush`] first, then use [`Self::into_parts`], or call
/// [`Self::into_inner`] to flush and return the wrapped writer in one step.
/// Dropping a `BufferedOutput` makes a best-effort attempt to write pending
/// buffered units, but drop-time errors are ignored. For arbitrary unit types,
/// `BufferedOutput` also supports [`Seekable`]-based seeking in unit offsets;
/// when `Item = u8` and the wrapped output is also [`std::io::Seek`], it
/// additionally implements [`std::io::Seek`].
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
    /// Creates a buffered unit output with the default capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The output that receives units when the internal buffer is
    ///   flushed.
    ///
    /// # Returns
    ///
    /// A new buffered unit output using `DEFAULT_BUFFER_CAPACITY`.
    #[inline(always)]
    #[must_use]
    pub fn new(inner: O) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered unit output with at least the requested capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The output that receives units when the internal buffer is
    ///   flushed.
    /// * `capacity` - The requested internal buffer capacity in units.
    ///
    /// # Returns
    ///
    /// A new buffered unit output whose actual buffer capacity is
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

    /// Returns a shared reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// An immutable reference to the underlying writer. Pending units may
    /// still be present in the internal buffer and are not flushed by this
    /// method.
    #[inline(always)]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped writer.
    ///
    /// Pending units may still be present in the internal buffer and are not
    /// flushed by this method.
    ///
    /// # Returns
    ///
    /// A mutable reference to the underlying writer.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Consumes this buffered output after flushing pending units.
    ///
    /// # Returns
    ///
    /// The wrapped output after all buffered units have been written.
    ///
    /// # Errors
    ///
    /// Returns any error produced while flushing pending units or flushing the
    /// wrapped output. If an error is returned, this value is dropped and any
    /// remaining pending units are flushed only on a best-effort basis.
    #[inline]
    pub fn into_inner(mut self) -> Result<O> {
        self.flush()?;
        let (inner, _) = self.into_parts();
        Ok(inner)
    }

    /// Consumes this buffered output without flushing pending units.
    ///
    /// This method performs no I/O. Pending units that have been accepted into
    /// the internal buffer but not written to the wrapped writer remain in the
    /// readable window of the returned buffer.
    ///
    /// # Returns
    ///
    /// The wrapped writer and the buffer holding pending units in logical write
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
    /// The total number of units that can be held by the internal buffer.
    #[inline(always)]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Returns the unused capacity in the internal buffer.
    ///
    /// # Returns
    ///
    /// The number of units that can still be appended to the internal buffer
    /// before it must be flushed.
    #[inline(always)]
    #[must_use]
    pub fn spare_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Returns raw spare-buffer parts for hot-path callers.
    ///
    /// The returned slice is the full internal backing storage. `index` is the
    /// start of the spare unit window, and `count` is the number of spare
    /// units. Callers that need a slice can use `&mut buffer[index..index +
    /// count]`; callers that already validated bounds can pass `buffer` and
    /// `index` directly to indexed unchecked codecs.
    ///
    /// Mutating units outside `index..index + count` changes pending output
    /// units and may corrupt the logical stream.
    ///
    /// # Returns
    ///
    /// The backing storage, the spare start index, and the spare unit count.
    #[inline(always)]
    #[must_use]
    pub fn spare_raw_parts_mut(&mut self) -> (&mut [O::Item], usize, usize) {
        self.buffer.spare_raw_parts_mut()
    }

    /// Marks spare units as written without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of initialized spare units to make pending for
    ///   output.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.spare_capacity()` and
    /// that the corresponding units in the spare range reported by
    /// [`Self::spare_raw_parts_mut`] have been initialized.
    #[inline(always)]
    pub unsafe fn advance(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` is within spare capacity.
        unsafe {
            self.buffer.advance(count);
        }
    }

    /// Ensures that at least `count` units are available in the spare buffer.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of spare units required.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// units. Returns [`ErrorKind::InvalidInput`] if `count` exceeds the buffer
    /// capacity. Returns [`ErrorKind::InvalidData`] if the wrapped writer
    /// reports more units than the pending buffer range contained.
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

    /// Writes units from the input slice and reports the accepted unit count.
    ///
    /// This is the buffered implementation for [`Write::write`]-style callers.
    /// Small inputs are appended to the buffer and reported as fully accepted;
    /// large inputs may be delegated to the wrapped writer after pending units
    /// are flushed.
    ///
    /// # Parameters
    ///
    /// * `input` - The units to write.
    ///
    /// # Returns
    ///
    /// The number of units accepted. Buffered writes return `input.len()`;
    /// direct writes return the unit count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending units or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero units were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more units than the requested range contained.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    #[inline]
    pub unsafe fn write_from(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), input_index, count),
            "unchecked write range exceeds input buffer"
        );
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

    /// Writes all units through the internal buffer.
    ///
    /// Small inputs are appended to the internal buffer.  Inputs that do not
    /// fit may flush the buffer first, and inputs at least as large as the
    /// buffer may be written directly to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - The units to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all units from `input` have been accepted.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending units or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero units were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more units than the requested range contained.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    #[inline]
    pub unsafe fn write_all_from(
        &mut self,
        input: &[O::Item],
        input_index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), input_index, count),
            "unchecked write range exceeds input buffer"
        );
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
            unsafe { self.write_all_cold(input, input_index, count) }
        }
    }

    /// Flushes buffered units to the wrapped writer.
    ///
    /// The method retries interrupted writes.  If an error occurs after some
    /// units have been written, the already-written units are removed from the
    /// front of the buffer and the unwritten suffix is kept for a later retry.
    ///
    /// # Returns
    ///
    /// `Ok(())` once all currently buffered units have been written to the
    /// wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped writer.
    /// Returns [`ErrorKind::WriteZero`] if the writer reports a zero-length
    /// write before all buffered units are drained. Returns
    /// [`ErrorKind::InvalidData`] if the writer reports more units than the
    /// pending buffer range contained.
    pub fn flush_buffer(&mut self) -> Result<()> {
        while !self.buffer.is_empty() {
            let position = self.buffer.position();
            let available = self.buffer.available();
            // SAFETY: `position..position + available` is the current readable
            // range maintained by `Buffer`.
            self.panicked = true;
            let result = unsafe { self.inner.write_from(self.buffer.data(), position, available) };
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
                    if let Err(error) = validate_write_count(written, available) {
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

    /// Flushes buffered units and then flushes the wrapped output.
    ///
    /// # Returns
    ///
    /// `Ok(())` once pending buffered units and the wrapped output are
    /// flushed.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// units, [`ErrorKind::WriteZero`] if the wrapped writer cannot make
    /// progress while draining the buffer, [`ErrorKind::InvalidData`] if the
    /// writer reports an impossible unit count, or any error returned by
    /// [`Write::flush`] on the wrapped writer.
    #[inline(always)]
    pub fn flush(&mut self) -> Result<()> {
        self.flush_buffer()
            .and_then(|()| Output::flush_pending(&mut self.inner))
    }

    /// Returns the logical output position without flushing pending units.
    ///
    /// The returned position is the wrapped output's current position plus the
    /// number of units currently pending in this buffer.
    ///
    /// # Returns
    ///
    /// The logical stream position in output units.
    ///
    /// # Errors
    ///
    /// Returns any error produced while querying the wrapped output position.
    /// Returns [`ErrorKind::InvalidData`] if adding the pending unit count
    /// overflows `u64`.
    #[inline]
    pub fn stream_position(&mut self) -> Result<u64>
    where
        O: SeekableOutput,
    {
        let position = Seekable::seek_to(&mut self.inner, SeekFrom::Current(0))?;
        position
            .checked_add(self.buffer.available() as u64)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    "buffered pending units overflow wrapped output position",
                )
            })
    }

    /// Seeks the wrapped output in units, flushing buffered units first.
    ///
    /// Pending units accepted into the internal buffer are written to the
    /// wrapped output before [`Seekable::seek_to`] is invoked, so the seek
    /// position is relative to data already committed to the underlying sink.
    ///
    /// # Parameters
    ///
    /// * `position` - The target seek position in output units.
    ///
    /// # Returns
    ///
    /// The new stream position in units.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// units, [`ErrorKind::WriteZero`] if the wrapped output cannot make
    /// progress while draining the buffer, [`ErrorKind::InvalidData`] if the
    /// writer reports an impossible unit count, or any error returned by
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

    /// Writes units into the internal buffer without checking spare capacity.
    ///
    /// # Parameters
    ///
    /// * `input` - The source units.
    /// * `input_index` - The starting index in `input`.
    /// * `count` - The number of units to copy.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `input_index..input_index + count` is valid
    /// in `input`, that `count <= self.spare_capacity()`, and that the copied
    /// source range does not overlap with the destination range in the internal
    /// buffer.
    #[inline(always)]
    unsafe fn write_to_buffer(&mut self, input: &[O::Item], input_index: usize, count: usize) {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), input_index, count),
            "unchecked write range exceeds input buffer"
        );
        debug_assert!(
            count <= self.spare_capacity(),
            "unchecked write exceeds spare buffer capacity"
        );
        let (destination, destination_index, _) = self.buffer.spare_raw_parts_mut();
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

    /// Writes units to the wrapped writer and validates the reported count.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `input_index` - Start index inside `input`.
    /// * `count` - Maximum number of units to write.
    ///
    /// # Returns
    ///
    /// The number of units accepted by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns the wrapped writer's I/O error, or [`ErrorKind::InvalidData`]
    /// if it reports a unit count larger than `count`.
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
        let written = unsafe { self.inner.write_from(input, input_index, count) }?;
        validate_write_count(written, count)?;
        Ok(written)
    }

    /// Writes all units in an indexed source range to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `input_index` - Start index inside `input`.
    /// * `count` - Number of units to write.
    ///
    /// # Errors
    ///
    /// Returns the wrapped writer's I/O error, [`ErrorKind::WriteZero`] if the
    /// writer cannot make progress, or [`ErrorKind::InvalidData`] if it
    /// reports an impossible unit count.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input` and that the addition does not overflow.
    unsafe fn write_all_inner(
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
            match unsafe { self.write_inner(input, input_index + written, remaining) } {
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
    /// * `input` - The units to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all units from `input` have been accepted either by the
    /// buffer or by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending units or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero units were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more units than the requested range contained.
    #[cold]
    #[inline(never)]
    unsafe fn write_all_cold(
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
            unsafe { self.write_all_inner(input, input_index, count) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer(input, input_index, count);
            }
            Ok(())
        }
    }

    /// Handles slow-path raw writes for [`Write::write`] semantics.
    ///
    /// The method preserves `Write::write` behavior: it may accept fewer units
    /// than the input length when the write is delegated directly to the
    /// wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - The units to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// The number of units accepted. Buffered writes return `input.len()`;
    /// direct writes return the unit count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending units or writing a
    /// large input directly to the wrapped writer. Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero units were
    /// written before the buffer is drained, and [`ErrorKind::InvalidData`] if
    /// it reports more units than the requested range contained.
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

impl<O> Write for BufferedOutput<O>
where
    O: Output<Item = u8>,
{
    /// Writes bytes through the internal buffer.
    #[inline(always)]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        // SAFETY: The full input slice is a valid source range.
        unsafe { BufferedOutput::write_from(self, buffer, 0, buffer.len()) }
    }

    /// Writes all bytes through the internal buffer.
    #[inline(always)]
    fn write_all(&mut self, buffer: &[u8]) -> Result<()> {
        // SAFETY: The full input slice is a valid source range.
        unsafe { BufferedOutput::write_all_from(self, buffer, 0, buffer.len()) }
    }

    /// Flushes the internal buffer and then the wrapped writer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        BufferedOutput::flush(self)
    }
}

impl<O> Seek for BufferedOutput<O>
where
    O: Output<Item = u8> + Seekable<Item = u8>,
{
    /// Flushes pending bytes before seeking the wrapped writer.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        BufferedOutput::seek_to(self, position)
    }

    /// Returns the logical byte position without flushing pending bytes.
    #[inline(always)]
    fn stream_position(&mut self) -> Result<u64> {
        BufferedOutput::stream_position(self)
    }
}

impl<O> Drop for BufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    fn drop(&mut self) {
        if !self.panicked {
            drop(self.flush());
        }
    }
}

/// Validates a unit count returned by a wrapped writer.
///
/// # Parameters
///
/// * `written` - Unit count reported by the wrapped writer.
/// * `requested` - Maximum unit count requested from the wrapped writer.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when the wrapped writer reports more
/// units than the source range contained.
#[inline(always)]
fn validate_write_count(written: usize, requested: usize) -> Result<()> {
    if written > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("writer reported {written} units for a {requested}-unit buffer"),
        ));
    }
    Ok(())
}
