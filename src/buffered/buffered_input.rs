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
/// build those concerns on top of [`Self::unread_slice`],
/// [`Self::unread_raw_parts`], [`Self::ensure_available`], and
/// [`Self::read_into_unchecked`]. The type also implements [`BufRead`] for
/// callers that want the standard buffered-read interface.
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
    /// unread bytes.
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
        let unread = self.unread_slice().to_vec();
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
    pub fn available(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the currently buffered unread units.
    ///
    /// # Returns
    ///
    /// The unread range `buffer[position..limit]`.
    #[inline(always)]
    #[must_use]
    pub fn unread_slice(&self) -> &[I::Item] {
        self.buffer.available_slice()
    }

    /// Returns raw unread-buffer parts for hot-path callers.
    ///
    /// The returned slice is the internal backing storage up to the unread
    /// tail. `index` is the start of the unread window, and `count` is the
    /// number of unread units. The returned range is valid for direct use
    /// with indexed unchecked codec operations that read from `index`.
    ///
    /// # Returns
    ///
    /// The backing storage, the unread start index, and the unread unit count.
    #[inline(always)]
    #[must_use]
    pub fn unread_raw_parts(&self) -> (&[I::Item], usize, usize) {
        self.buffer.available_raw_parts()
    }

    /// Advances the unread cursor by `count` units.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of currently unread units to consume.
    ///
    /// # Panics
    ///
    /// Panics when `count` exceeds [`Self::available`].
    #[inline(always)]
    pub fn consume(&mut self, count: usize) {
        assert!(
            count <= self.available(),
            "cannot consume beyond buffered input"
        );
        // SAFETY: The assertion proves that `count` is within the readable
        // input window.
        unsafe {
            self.buffer.consume_unchecked(count);
        }
    }

    /// Advances the unread cursor without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of currently unread bytes to consume.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.available()`.
    #[inline(always)]
    pub unsafe fn consume_unchecked(&mut self, count: usize) {
        // SAFETY: The caller guarantees that `count` is within the readable
        // input window.
        unsafe {
            self.buffer.consume_unchecked(count);
        }
    }

    /// Refills the internal buffer after preserving unread bytes.
    ///
    /// Consumed bytes may be discarded, and unread bytes may be moved to the
    /// front of the buffer before the wrapped reader is called.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least one byte was appended, or `Ok(false)` at EOF.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    pub fn fill_more(&mut self) -> Result<bool> {
        if self.available() == 0 {
            self.discard_buffer();
        } else if self.tail_capacity() == 0 {
            self.backshift();
        }
        self.read_more()
    }

    /// Refills the buffer until at least `count` unread bytes are available.
    ///
    /// This method may discard consumed bytes or move unread bytes to the front
    /// of the buffer before reading more data. It stops as soon as the unread
    /// window reaches `count` bytes or the wrapped reader reaches EOF.
    ///
    /// # Parameters
    ///
    /// * `count` - Minimum number of unread bytes required.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least `count` unread bytes are buffered. `Ok(false)`
    /// means EOF was reached before the requested byte count became available.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the internal
    /// buffer capacity. Returns [`ErrorKind::InvalidData`] if the wrapped
    /// reader reports more bytes than the spare buffer range could hold.
    /// Returns any non-interrupted I/O error produced by the wrapped reader
    /// while refilling the buffer.
    #[inline]
    pub fn fill_until(&mut self, count: usize) -> Result<bool> {
        if count > self.capacity() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "requested available bytes exceed buffered input capacity",
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

    /// Ensures that at least `count` unread bytes are available.
    ///
    /// Unlike [`Self::fill_until`], this method treats EOF before the requested
    /// byte count as [`ErrorKind::UnexpectedEof`]. Any partial bytes buffered
    /// before EOF are consumed so callers observe the same logical position as
    /// a failed exact read.
    ///
    /// # Parameters
    ///
    /// * `count` - Minimum number of unread bytes required.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before `count`
    /// bytes are available. Returns [`ErrorKind::InvalidInput`] when `count`
    /// exceeds the internal buffer capacity. Returns [`ErrorKind::InvalidData`]
    /// if the wrapped reader reports more bytes than the spare buffer range
    /// could hold. Returns any non-interrupted I/O error produced by the
    /// wrapped reader while refilling the buffer.
    #[inline]
    pub fn ensure_available(&mut self, count: usize) -> Result<()> {
        if self.fill_until(count)? {
            return Ok(());
        }
        let available = self.available();
        // SAFETY: `available` is the current readable byte count.
        unsafe {
            self.consume_unchecked(available);
        }
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    }

    /// Reads bytes through the internal buffer into an indexed output range.
    ///
    /// If the internal buffer is empty and `count` is at least as large as the
    /// internal buffer capacity, the read is delegated directly to the wrapped
    /// reader to avoid an unnecessary copy. Otherwise, bytes are served from
    /// the internal buffer.
    ///
    /// # Arguments
    ///
    /// * `output` - Destination storage that receives bytes.
    /// * `output_index` - Start index inside `output`.
    /// * `count` - Maximum number of bytes to read.
    ///
    /// # Returns
    ///
    /// The number of bytes written into `output[output_index..output_index +
    /// count]`. A return value of `0` means that `count` was zero or EOF was
    /// reached before any bytes were read.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced by the wrapped reader. Returns
    /// [`ErrorKind::InvalidData`] if the wrapped reader reports more bytes
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
    pub unsafe fn read_into_unchecked(
        &mut self,
        output: &mut [I::Item],
        output_index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            output_index
                .checked_add(count)
                .is_some_and(|end| end <= output.len()),
            "unchecked read output range exceeds destination buffer"
        );
        if count == 0 {
            return Ok(0);
        }
        if self.available() == 0 {
            self.discard_buffer();
            if count >= self.buffer.capacity() {
                // SAFETY: The caller guarantees that the target range is valid.
                let read = unsafe {
                    self.inner.read_unchecked(output, output_index, count)
                }?;
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
            self.buffer
                .copy_to_unchecked(output, output_index, read_count);
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
        let position = match position {
            SeekFrom::Current(offset) => {
                // Unread units fit in `isize` for any `Vec`-backed buffer,
                // which always fits in `i64`.
                let unread = self.available() as i64;
                let adjusted = offset.checked_sub(unread).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "current seek offset underflows after buffered adjustment",
                    )
                })?;
                Seekable::seek(&mut self.inner, SeekFrom::Current(adjusted))
            }
            other => Seekable::seek(&mut self.inner, other),
        }?;
        self.discard_buffer();
        Ok(position)
    }

    /// Returns the unused capacity at the end of the buffer.
    ///
    /// # Returns
    ///
    /// The number of writable bytes in `buffer[limit..]`.
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
    /// future reads. If there are no unread bytes, the buffer is discarded.
    #[inline(always)]
    fn backshift(&mut self) {
        self.buffer.compact();
    }

    /// Appends one more chunk from the wrapped reader to the internal buffer.
    ///
    /// This method reads into `buffer[limit..]` and advances `limit` by the
    /// number of bytes read. It retries automatically when the wrapped reader
    /// returns [`ErrorKind::Interrupted`].
    ///
    /// # Returns
    ///
    /// `Ok(true)` if at least one byte was appended, or `Ok(false)` if the
    /// wrapped reader reached EOF.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped reader.
    /// Returns [`ErrorKind::InvalidData`] if the wrapped reader reports more
    /// bytes than the spare buffer range could hold.
    fn read_more(&mut self) -> Result<bool> {
        let count = self.tail_capacity();
        debug_assert!(count > 0, "buffer has no tail capacity");
        loop {
            let limit = self.buffer.limit();
            // SAFETY: `limit` is always within `buffer`, and `count` is the
            // remaining capacity from `limit` to the end of `buffer`.
            match unsafe {
                self.inner
                    .read_unchecked(self.buffer.data_mut(), limit, count)
            } {
                Ok(0) => return Ok(false),
                Ok(read) => {
                    validate_read_count(read, count)?;
                    // SAFETY: `read_unchecked` returns a count in
                    // `0..=count`, and `count` was the spare capacity.
                    unsafe {
                        self.buffer.advance_unchecked(read);
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
        unsafe { self.read_into_unchecked(output, 0, output.len()) }
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
        Ok(self.unread_slice())
    }

    /// Consumes `amount` bytes from the unread byte window.
    #[inline(always)]
    fn consume(&mut self, amount: usize) {
        BufferedInput::consume(self, amount);
    }
}

impl<I> Seek for BufferedInput<I>
where
    I: Input<Item = u8> + Seekable<Item = u8>,
{
    /// Seeks the wrapped reader and discards buffered bytes after success.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        BufferedInput::seek(self, position)
    }
}

/// Validates a byte count returned by a wrapped reader.
///
/// # Parameters
///
/// * `read` - Byte count reported by the wrapped reader.
/// * `requested` - Maximum byte count requested from the wrapped reader.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when the wrapped reader reports more
/// bytes than the destination range could hold.
#[inline(always)]
fn validate_read_count(read: usize, requested: usize) -> Result<()> {
    if read > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "reader reported {read} bytes for a {requested}-byte buffer"
            ),
        ));
    }
    Ok(())
}
