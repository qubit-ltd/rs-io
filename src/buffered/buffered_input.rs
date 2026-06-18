// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    BufRead,
    Error,
    ErrorKind,
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::buffered::DEFAULT_BUFFER_CAPACITY;
use crate::util::{
    UncheckedSlice,
};
use crate::{
    Buffer,
    Input,
    Seekable,
    SeekableInput,
};

/// Buffered unit input over a wrapped input source.
///
/// This type owns a wrapped input object and an internal unit buffer. It keeps
/// unread units in `buffer[position..limit]` so callers can inspect or consume
/// the current unit window before refilling it.
///
/// `BufferedInput` is deliberately unit-oriented. It performs no binary
/// decoding, text decoding, or record parsing; higher-level stream adapters can
/// build those concerns on top of [`Self::ensure_available`],
/// [`Self::copy_unread_to`], and [`Self::read_into`]. The type also implements
/// [`BufRead`] for callers that want the standard
/// buffered-read interface.
#[derive(Debug)]
pub struct BufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    inner: I,
    buffer: Buffer<I::Item>,
}

impl<I> BufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// Creates a buffered unit input with the default capacity.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    ///
    /// # Returns
    ///
    /// A new buffered unit input whose internal buffer has at least
    /// `DEFAULT_BUFFER_CAPACITY` units.
    #[inline(always)]
    #[must_use]
    pub fn new(inner: I) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered unit input with at least the requested capacity.
    ///
    /// The actual capacity is raised to `1` when the requested value is `0`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    /// * `capacity` - The requested internal buffer capacity, in units.
    ///
    /// # Returns
    ///
    /// A new buffered unit input whose internal buffer capacity is
    /// `capacity.max(1)`.
    #[inline]
    #[must_use]
    pub fn with_capacity(inner: I, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Returns a shared reference to the wrapped input object.
    ///
    /// # Returns
    ///
    /// A shared reference to the inner input object.
    #[inline(always)]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped input object.
    ///
    /// Mutating the wrapped object directly may invalidate assumptions about
    /// units already buffered by this value.
    ///
    /// # Returns
    ///
    /// An exclusive reference to the wrapped input object.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Consumes this buffered input and returns the wrapped input object plus
    /// unread units.
    ///
    /// This method performs no I/O. Units that have already been read from the
    /// wrapped input but not consumed by this buffered input are returned as
    /// the second tuple item.
    ///
    /// # Returns
    ///
    /// The wrapped input object and a vector containing the unread buffered
    /// units in logical read order.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, Vec<I::Item>) {
        let unread = self.buffer.readable().to_vec();
        (self.inner, unread)
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

    /// Returns the number of unread units currently buffered.
    ///
    /// # Returns
    ///
    /// The length of `buffer[position..limit]`, in units.
    #[inline(always)]
    #[must_use]
    pub const fn available(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the currently buffered unread units.
    ///
    /// # Returns
    ///
    /// The `buffer[position..limit]` unread unit window. The slice may be empty
    /// when no units are currently buffered.
    #[inline(always)]
    #[must_use]
    pub fn unread(&self) -> &[I::Item] {
        self.buffer.readable()
    }

    /// Advances the unread cursor without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of currently unread units to consume.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.available()`.
    #[inline(always)]
    pub unsafe fn consume(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` is within the readable
        // input window.
        unsafe {
            self.buffer.consume(count);
        }
    }

    /// Copies unread units into an indexed output range without consuming them.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage that receives a copy of unread units.
    /// * `output_index` - Start index inside `output`.
    /// * `count` - Number of unread units to copy.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output`, that the addition does not overflow, that
    /// `count <= self.available()`, and that the destination range does not
    /// overlap with the unread range stored inside this buffer.
    #[inline(always)]
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
            count <= self.available(),
            "unchecked unread copy exceeds available input buffer",
        );
        // SAFETY: The caller guarantees that the destination range is valid,
        // non-overlapping, and that `count` unread units are currently
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

    /// Refills the internal buffer after preserving unread units.
    ///
    /// Consumed units may be discarded, and unread units may be moved to the
    /// front of the buffer before the wrapped reader is called.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least one unit was appended, or `Ok(false)` at EOF.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    /// Returns [`ErrorKind::InvalidInput`] when the buffer is already full and
    /// no unread units have been consumed; callers must consume buffered units
    /// before refilling in that state.
    pub fn fill_more(&mut self) -> Result<bool> {
        if self.available() == 0 {
            self.discard_buffer();
        } else if self.tail_capacity() == 0 {
            self.backshift();
            if self.tail_capacity() == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "buffered input is full; consume buffered units before refilling",
                ));
            }
        }
        self.read_more()
    }

    /// Refills the buffer until at least `count` unread units are available.
    ///
    /// This method may discard consumed units or move unread units to the front
    /// of the buffer before reading more data. It stops as soon as the unread
    /// window reaches `count` units or the wrapped reader reaches EOF.
    ///
    /// # Parameters
    ///
    /// * `count` - Minimum number of unread units required.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least `count` unread units are buffered. `Ok(false)`
    /// means EOF was reached before the requested unit count became available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the internal
    /// buffer capacity. Returns [`ErrorKind::InvalidData`] if the wrapped
    /// reader reports more units than the spare buffer range could hold.
    /// Returns any non-interrupted I/O error produced by the wrapped reader
    /// while refilling the buffer.
    #[inline]
    pub fn fill_until(&mut self, count: usize) -> Result<bool> {
        if count > self.capacity() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "requested available units exceed buffered input capacity",
            ));
        }
        while self.available() < count {
            let available = self.available();
            if available == 0 {
                self.discard_buffer();
            } else {
                let missing = count - available;
                if self.tail_capacity() < missing {
                    self.backshift();
                }
            }
            if !self.read_more()? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Ensures that at least `count` unread units are available.
    ///
    /// Unlike [`Self::fill_until`], this method treats EOF before the requested
    /// unit count as [`ErrorKind::UnexpectedEof`]. Any partial units buffered
    /// before EOF are consumed so callers observe the same logical position as
    /// a failed exact read.
    ///
    /// # Parameters
    ///
    /// * `count` - Minimum number of unread units required.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before `count`
    /// units are available. Returns [`ErrorKind::InvalidInput`] when `count`
    /// exceeds the internal buffer capacity. Returns [`ErrorKind::InvalidData`]
    /// if the wrapped reader reports more units than the spare buffer range
    /// could hold. Returns any non-interrupted I/O error produced by the
    /// wrapped reader while refilling the buffer.
    #[inline]
    pub fn ensure_available(&mut self, count: usize) -> Result<()> {
        if self.fill_until(count)? {
            return Ok(());
        }
        let available = self.available();
        // SAFETY: `available` is the current readable unit count.
        unsafe {
            self.consume(available);
        }
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    }

    /// Reads units through the internal buffer into an indexed output range.
    ///
    /// If the internal buffer is empty and `count` is at least as large as the
    /// internal buffer capacity, the read is delegated directly to the wrapped
    /// reader to avoid an unnecessary copy. Otherwise, units are served from
    /// the internal buffer.
    ///
    /// # Arguments
    ///
    /// * `output` - Destination storage that receives units.
    /// * `output_index` - Start index inside `output`.
    /// * `count` - Maximum number of units to read.
    ///
    /// # Returns
    ///
    /// The number of units written into `output[output_index..output_index +
    /// count]`. A return value of `0` means that `count` was zero or EOF was
    /// reached before any units were read.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced by the wrapped reader. Returns
    /// [`ErrorKind::InvalidData`] if the wrapped reader reports more units
    /// than the requested destination range could hold. Interrupted reads are
    /// retried when the method refills the internal buffer through
    /// `read_more`; direct delegated reads follow the wrapped reader's own
    /// [`Read::read`] behavior.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output` and that the addition does not overflow.
    #[inline(always)]
    pub unsafe fn read_into(
        &mut self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), output_index, count),
            "unchecked read output range exceeds destination buffer"
        );
        if count == 0 {
            return Ok(0);
        }
        if self.available() == 0 {
            self.discard_buffer();
            if count >= self.buffer.capacity() {
                // SAFETY: The caller guarantees that the target range is valid.
                let read =
                    unsafe { self.inner.read(output, output_index, count) }?;
                validate_read_count(read, count)?;
                return Ok(read);
            }
            if !self.read_more()? {
                return Ok(0);
            }
        }
        let read_count = count.min(self.available());
        // SAFETY: `read_count` is bounded by the caller-provided output range
        // and the available input range.
        unsafe {
            self.buffer.copy_to(output, output_index, read_count);
        }
        Ok(read_count)
    }

    /// Seeks the wrapped reader and discards buffered units after success.
    ///
    /// For [`SeekFrom::Current`], the offset is adjusted by the number of
    /// unread units already buffered, so seeking is relative to the logical
    /// position observed by callers of this buffered input.
    ///
    /// # Arguments
    ///
    /// * `position` - The target seek position.
    ///
    /// # Returns
    ///
    /// The new absolute stream position reported by the wrapped reader, in
    /// units.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if a [`SeekFrom::Current`] offset
    /// cannot be adjusted by the unread buffered unit count. Returns any seek
    /// error produced by the wrapped reader.
    pub fn seek(&mut self, position: SeekFrom) -> Result<u64>
    where
        I: SeekableInput,
    {
        match position {
            SeekFrom::Current(offset) => {
                if self.seek_within_buffer(offset) {
                    return self.logical_stream_position();
                }
                let position = self.seek_relative_slow(offset)?;
                self.discard_buffer();
                Ok(position)
            }
            other => {
                let position = Seekable::seek(&mut self.inner, other)?;
                self.discard_buffer();
                Ok(position)
            }
        }
    }

    /// Moves the logical position relative to the current buffered position.
    ///
    /// If the target remains within the current backing buffer, only the buffer
    /// cursor is moved and the wrapped input is not sought. Otherwise the seek
    /// is delegated to the wrapped input and the buffer is discarded.
    ///
    /// # Parameters
    ///
    /// * `offset` - Relative offset in input units.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if the offset cannot be adjusted by
    /// the unread buffered unit count. Returns any seek error produced by the
    /// wrapped reader.
    fn seek_relative(&mut self, offset: i64) -> Result<()>
    where
        I: SeekableInput,
    {
        if self.seek_within_buffer(offset) {
            return Ok(());
        }
        self.seek_relative_slow(offset)?;
        self.discard_buffer();
        Ok(())
    }

    /// Returns the logical stream position without discarding buffered units.
    ///
    /// # Returns
    ///
    /// The wrapped input position minus the unread buffered unit count.
    ///
    /// # Errors
    ///
    /// Returns any seek error produced while querying the wrapped input's
    /// current position. Returns [`ErrorKind::InvalidData`] if the wrapped
    /// input reports a position before the unread buffered window.
    fn logical_stream_position(&mut self) -> Result<u64>
    where
        I: SeekableInput,
    {
        let position = Seekable::seek(&mut self.inner, SeekFrom::Current(0))?;
        let unread = self.available() as u64;
        position.checked_sub(unread).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "buffered unread units exceed wrapped input position",
            )
        })
    }

    /// Seeks the wrapped input relative to the logical current position.
    ///
    /// # Parameters
    ///
    /// * `offset` - Relative offset in input units.
    ///
    /// # Returns
    ///
    /// The new position reported by the wrapped input.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if `offset` cannot be adjusted by
    /// the unread buffered unit count. Returns any seek error produced by
    /// the wrapped input.
    fn seek_relative_slow(&mut self, offset: i64) -> Result<u64>
    where
        I: SeekableInput,
    {
        // Unread units fit in `isize` for any `Vec`-backed buffer, which always
        // fits in `i64`.
        let unread = self.available() as i64;
        let adjusted = offset.checked_sub(unread).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "current seek offset underflows after buffered adjustment",
            )
        })?;
        Seekable::seek(&mut self.inner, SeekFrom::Current(adjusted))
    }

    /// Attempts to satisfy a relative seek inside the current buffer window.
    ///
    /// Positive offsets consume unread units. Negative offsets can rewind into
    /// the still-retained consumed prefix of the backing buffer. If the target
    /// is outside the retained buffer contents, the caller must seek the
    /// wrapped input instead.
    ///
    /// # Parameters
    ///
    /// * `offset` - Relative offset in input units.
    ///
    /// # Returns
    ///
    /// `true` when the buffer cursor was moved. `false` when the caller must
    /// delegate the seek to the wrapped input.
    fn seek_within_buffer(&mut self, offset: i64) -> bool {
        if offset >= 0 {
            let count = offset as u64;
            if count <= self.available() as u64 {
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

    /// Returns the unused capacity at the end of the buffer.
    ///
    /// # Returns
    ///
    /// The number of writable units in `buffer[limit..]`.
    #[inline(always)]
    fn tail_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Invalidates all buffered units.
    ///
    /// After this call, the buffer is considered empty and subsequent reads
    /// will refill it from the wrapped input.
    #[inline(always)]
    fn discard_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Moves unread units to the front of the buffer.
    ///
    /// This preserves the unread range while reclaiming tail capacity for
    /// future reads. If there are no unread units, the buffer is discarded.
    #[inline(always)]
    fn backshift(&mut self) {
        self.buffer.compact();
    }

    /// Appends one more chunk from the wrapped reader to the internal buffer.
    ///
    /// This method reads into `buffer[limit..]` and advances `limit` by the
    /// number of units read. It retries automatically when the wrapped reader
    /// returns [`ErrorKind::Interrupted`].
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least one unit was appended, or `Ok(false)` if the
    /// wrapped reader reached EOF.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    /// Returns [`ErrorKind::InvalidData`] if the wrapped reader reports more
    /// units than the spare buffer range could hold.
    fn read_more(&mut self) -> Result<bool> {
        let count = self.tail_capacity();
        debug_assert!(count > 0, "buffer has no tail capacity");
        loop {
            let limit = self.buffer.limit();
            // SAFETY: `limit` is always within `buffer`, and `count` is the
            // remaining capacity from `limit` to the end of `buffer`.
            match unsafe {
                self.inner.read(self.buffer.data_mut(), limit, count)
            } {
                Ok(0) => return Ok(false),
                Ok(read) => {
                    validate_read_count(read, count)?;
                    // SAFETY: `read` returns a count in
                    // `0..=count`, and `count` was the spare capacity.
                    unsafe {
                        self.buffer.advance(read);
                    }
                    return Ok(true);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl<I> Read for BufferedInput<I>
where
    I: Input<Item = u8>,
{
    /// Reads bytes through the internal buffer.
    ///
    /// # Arguments
    ///
    /// * `output` - Destination slice that receives the bytes read.
    ///
    /// # Returns
    ///
    /// The number of bytes written to `output`.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced by the wrapped reader.
    #[inline(always)]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        // SAFETY: The full output slice is a valid writable range.
        unsafe { self.read_into(output, 0, output.len()) }
    }
}

impl<I> BufRead for BufferedInput<I>
where
    I: Input<Item = u8>,
{
    /// Returns the currently buffered unread bytes, refilling when empty.
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.available() == 0 {
            self.discard_buffer();
            if !self.read_more()? {
                return Ok(&[]);
            }
        }
        Ok(self.buffer.readable())
    }

    /// Consumes `amount` bytes from the unread byte window.
    #[inline(always)]
    fn consume(&mut self, amount: usize) {
        assert!(
            amount <= BufferedInput::available(self),
            "cannot consume beyond buffered input"
        );
        // SAFETY: The assertion proves that `amount` is within the readable
        // input window.
        unsafe {
            BufferedInput::consume(self, amount);
        }
    }
}

impl<I> Seek for BufferedInput<I>
where
    I: Input<Item = u8> + Seekable<Item = u8>,
{
    /// Seeks the wrapped reader and discards buffered bytes after success.
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        BufferedInput::seek(self, position)
    }

    /// Returns the logical stream position without discarding buffered bytes.
    #[inline(always)]
    fn stream_position(&mut self) -> Result<u64> {
        self.logical_stream_position()
    }

    /// Seeks relative to the current logical position.
    #[inline(always)]
    fn seek_relative(&mut self, offset: i64) -> Result<()> {
        BufferedInput::seek_relative(self, offset)
    }
}

/// Validates a unit count returned by a wrapped reader.
///
/// # Parameters
///
/// * `read` - Unit count reported by the wrapped reader.
/// * `requested` - Maximum unit count requested from the wrapped reader.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when the wrapped reader reports more
/// units than the destination range could hold.
#[inline(always)]
fn validate_read_count(read: usize, requested: usize) -> Result<()> {
    if read > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "reader reported {read} units for a {requested}-unit buffer"
            ),
        ));
    }
    Ok(())
}
