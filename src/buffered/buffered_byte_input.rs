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
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::Buffer;
use crate::ReadExt;
use crate::buffered::DEFAULT_BUFFER_CAPACITY;

/// Buffered byte input over a wrapped reader.
///
/// This type owns a wrapped input object and an internal byte buffer. It keeps
/// unread bytes in `buffer[position..limit]` so callers can inspect or consume
/// the current byte window before refilling it.
///
/// `BufferedByteInput` is deliberately byte-oriented. It performs no binary
/// decoding, text decoding, or record parsing; higher-level stream adapters can
/// build those concerns on top of [`Self::unread_slice`],
/// [`Self::ensure_available`], and [`Self::read_into_unchecked`].
#[derive(Debug)]
pub struct BufferedByteInput<R> {
    inner: R,
    buffer: Buffer<u8>,
}

impl<R> BufferedByteInput<R> {
    /// Creates a buffered byte input with the default capacity.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    ///
    /// # Returns
    ///
    /// A new buffered byte input whose internal buffer has at least
    /// `DEFAULT_BUFFER_CAPACITY` bytes.
    #[inline(always)]
    #[must_use]
    pub fn new(inner: R) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered byte input with at least the requested capacity.
    ///
    /// The actual capacity is raised to `1` when the requested value is `0`.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    /// * `capacity` - The requested internal buffer capacity, in bytes.
    ///
    /// # Returns
    ///
    /// A new buffered byte input whose internal buffer capacity is
    /// `capacity.max(1)`.
    #[inline]
    #[must_use]
    pub fn with_capacity(inner: R, capacity: usize) -> Self {
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
    pub const fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped input object.
    ///
    /// Mutating the wrapped object directly may invalidate assumptions about
    /// bytes already buffered by this value.
    ///
    /// # Returns
    ///
    /// An exclusive reference to the wrapped input object.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this buffered input and returns the wrapped input object.
    ///
    /// Any unread bytes currently held in the internal buffer are discarded.
    ///
    /// # Returns
    ///
    /// The wrapped input object.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the internal buffer capacity.
    ///
    /// # Returns
    ///
    /// The total number of bytes that can be held by the internal buffer.
    #[inline(always)]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Returns the number of unread bytes currently buffered.
    ///
    /// # Returns
    ///
    /// The length of `buffer[position..limit]`, in bytes.
    #[inline(always)]
    #[must_use]
    pub fn available(&self) -> usize {
        self.buffer.available()
    }

    /// Returns the currently buffered unread bytes.
    ///
    /// # Returns
    ///
    /// The unread range `buffer[position..limit]`.
    #[inline(always)]
    #[must_use]
    pub fn unread_slice(&self) -> &[u8] {
        &self.buffer.data()[self.buffer.position()..self.buffer.limit()]
    }

    /// Advances the unread cursor by `count` bytes.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of currently unread bytes to consume.
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

    /// Returns the unused capacity at the end of the buffer.
    ///
    /// # Returns
    ///
    /// The number of writable bytes in `buffer[limit..]`.
    #[inline(always)]
    fn tail_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Invalidates all buffered bytes.
    ///
    /// After this call, the buffer is considered empty and subsequent reads
    /// will refill it from the wrapped reader.
    #[inline(always)]
    fn discard_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Moves unread bytes to the front of the buffer.
    ///
    /// This preserves the unread range while reclaiming tail capacity for
    /// future reads. If there are no unread bytes, the buffer is discarded.
    #[inline(always)]
    fn backshift(&mut self) {
        self.buffer.compact();
    }
}

impl<R> BufferedByteInput<R>
where
    R: Read,
{
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
    /// buffer capacity. Returns any non-interrupted I/O error produced by the
    /// wrapped reader while refilling the buffer.
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
    /// exceeds the internal buffer capacity. Returns any non-interrupted I/O
    /// error produced by the wrapped reader while refilling the buffer.
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
    /// Returns any I/O error produced by the wrapped reader. Interrupted reads
    /// are retried when the method refills the internal buffer through
    /// `read_more`; direct delegated reads follow the wrapped reader's
    /// own [`Read::read`] behavior.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output` and that the addition does not overflow.
    #[inline(always)]
    pub unsafe fn read_into_unchecked(
        &mut self,
        output: &mut [u8],
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
                return unsafe {
                    self.inner.read_unchecked(output, output_index, count)
                };
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

    /// Seeks the wrapped reader and discards buffered bytes after success.
    ///
    /// For [`SeekFrom::Current`], the offset is adjusted by the number of
    /// unread bytes already buffered, so seeking is relative to the logical
    /// position observed by callers of this buffered input.
    ///
    /// # Arguments
    ///
    /// * `position` - The target seek position.
    ///
    /// # Returns
    ///
    /// The new absolute stream position reported by the wrapped reader.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if a [`SeekFrom::Current`] offset
    /// cannot be adjusted by the unread buffered byte count. Returns any seek
    /// error produced by the wrapped reader.
    fn seek_logical(&mut self, position: SeekFrom) -> Result<u64>
    where
        R: Seek,
    {
        let position = match position {
            SeekFrom::Current(offset) => {
                // `buffer` is a `Vec<u8>`, whose maximum allocation size fits
                // in `isize`; that always fits in `i64`.
                let unread = self.available() as i64;
                let adjusted = offset.checked_sub(unread).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "current seek offset underflows after buffered adjustment",
                    )
                })?;
                self.inner.seek(SeekFrom::Current(adjusted))
            }
            other => self.inner.seek(other),
        }?;
        self.discard_buffer();
        Ok(position)
    }
}

impl<R> Read for BufferedByteInput<R>
where
    R: Read,
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

impl<R> Seek for BufferedByteInput<R>
where
    R: Read + Seek,
{
    /// Seeks the wrapped reader and discards buffered bytes after success.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.seek_logical(position)
    }
}
