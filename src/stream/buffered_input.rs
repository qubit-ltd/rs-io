/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::ReadExt;

/// Default capacity used by buffered codec readers and writers.
pub(crate) const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum capacity required by the largest scalar codec payload.
pub(crate) const MIN_CODEC_BUFFER_CAPACITY: usize = 19;

/// Buffered input core shared by codec-oriented readers.
///
/// This type owns a wrapped input object and an internal byte buffer. It keeps
/// unread bytes in `buffer[position..limit]` so codec readers can decode scalar
/// values without repeatedly allocating temporary storage.
pub(crate) struct BufferedInput<R> {
    inner: R,
    buffer: Vec<u8>,
    position: usize,
    limit: usize,
}

impl<R> BufferedInput<R> {
    /// Creates a buffered input core with the default capacity.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    ///
    /// # Returns
    ///
    /// A new buffered input whose internal buffer has at least
    /// [`DEFAULT_BUFFER_CAPACITY`] bytes.
    #[inline]
    pub(crate) fn new(inner: R) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered input core with at least the requested capacity.
    ///
    /// The actual capacity is raised to [`MIN_CODEC_BUFFER_CAPACITY`] when the
    /// requested value is smaller, so every scalar codec payload can fit in the
    /// buffer.
    ///
    /// # Arguments
    ///
    /// * `inner` - The input object wrapped by this buffer.
    /// * `capacity` - The requested internal buffer capacity, in bytes.
    ///
    /// # Returns
    ///
    /// A new buffered input whose internal buffer capacity is
    /// `capacity.max(MIN_CODEC_BUFFER_CAPACITY)`.
    #[inline]
    pub(crate) fn with_capacity(inner: R, capacity: usize) -> Self {
        let capacity = capacity.max(MIN_CODEC_BUFFER_CAPACITY);
        Self {
            inner,
            buffer: vec![0; capacity],
            position: 0,
            limit: 0,
        }
    }

    /// Returns a shared reference to the wrapped input object.
    ///
    /// # Returns
    ///
    /// A shared reference to the inner input object.
    #[inline]
    pub(crate) const fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped input object.
    ///
    /// Mutating the inner object directly may invalidate assumptions about the
    /// bytes already buffered by this value. Callers must keep the buffered
    /// state and the wrapped input position consistent.
    ///
    /// # Returns
    ///
    /// An exclusive reference to the inner input object.
    #[inline]
    pub(crate) fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this buffered input and returns the wrapped input object.
    ///
    /// Any unread bytes currently held in the internal buffer are discarded.
    ///
    /// # Returns
    ///
    /// The wrapped input object.
    #[inline]
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the number of unread bytes currently buffered.
    ///
    /// # Returns
    ///
    /// The length of `buffer[position..limit]`, in bytes.
    #[inline]
    fn available(&self) -> usize {
        self.limit - self.position
    }

    /// Returns the unused capacity at the end of the buffer.
    ///
    /// # Returns
    ///
    /// The number of writable bytes in `buffer[limit..]`.
    #[inline]
    fn tail_capacity(&self) -> usize {
        self.buffer.len() - self.limit
    }

    /// Invalidates all buffered bytes.
    ///
    /// After this call, the buffer is considered empty and subsequent reads will
    /// refill it from the wrapped reader.
    #[inline]
    fn discard_buffer(&mut self) {
        self.position = 0;
        self.limit = 0;
    }

    /// Moves unread bytes to the front of the buffer.
    ///
    /// This preserves the unread range while reclaiming tail capacity for
    /// future reads. If there are no unread bytes, the buffer is discarded.
    #[inline]
    fn backshift(&mut self) {
        self.buffer.copy_within(self.position..self.limit, 0);
        self.limit -= self.position;
        self.position = 0;
    }
}

impl<R> BufferedInput<R>
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
            // SAFETY: `limit` is always within `buffer`, and `count` is the
            // remaining capacity from `limit` to the end of `buffer`.
            match unsafe { self.inner.read_unchecked(&mut self.buffer, self.limit, count) } {
                Ok(0) => return Ok(false),
                Ok(read) => {
                    self.limit += read;
                    return Ok(true);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }

    /// Ensures that at least `count` unread bytes are available.
    ///
    /// The method may discard consumed bytes or move unread bytes to the front
    /// of the buffer before reading more data.
    ///
    /// # Arguments
    ///
    /// * `count` - The minimum number of unread bytes required in the buffer.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before `count`
    /// bytes are available. Returns any non-interrupted I/O error produced by
    /// the wrapped reader while refilling the buffer.
    fn ensure_available_slow(&mut self, count: usize) -> Result<()> {
        debug_assert!(count <= self.buffer.len(), "requested range exceeds buffer capacity");
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
                self.position = self.limit;
                return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
            }
        }
        Ok(())
    }

    /// Reads one fixed-width value directly from the internal buffer.
    ///
    /// The method ensures that `N` bytes are buffered, calls `decode` at the
    /// current buffer position, and then advances the position by `N` bytes.
    ///
    /// # Type Parameters
    ///
    /// * `N` - The exact number of bytes consumed by the fixed-width value.
    /// * `T` - The decoded value type.
    /// * `F` - The decoder function type.
    ///
    /// # Arguments
    ///
    /// * `decode` - Function that decodes a value from the internal buffer and
    ///   the starting index of the value.
    ///
    /// # Returns
    ///
    /// The decoded value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before `N` bytes
    /// are available. Returns any non-interrupted I/O error produced by the
    /// wrapped reader while refilling the buffer.
    #[inline]
    pub(crate) fn read_fixed<const N: usize, T, F>(&mut self, decode: F) -> Result<T>
    where
        F: FnOnce(&[u8], usize) -> T,
    {
        debug_assert!(N <= self.buffer.len(), "requested range exceeds buffer capacity");
        if self.available() < N {
            self.ensure_available_slow(N)?;
        }
        let index = self.position;
        let value = decode(&self.buffer, index);
        // Keep the cursor update based on the saved `index` instead of
        // writing `self.position += N`. This fixed-width read path is hot
        // enough that the exact expression shape has measured impact: using
        // `index + N` states the real invariant directly, namely that the
        // cursor advances from the position that was checked before `decode`
        // ran. Re-reading and incrementing `self.position` after the callback
        // makes the optimizer reason about the field again and was slower in
        // the production-style binary read benchmark.
        self.position = index + N;
        Ok(value)
    }

    /// Reads one variable-width value while the decoder scans available bytes.
    ///
    /// The method calls `decode_available` with the unread byte range currently
    /// available in the internal buffer, capped at `N`. The decoder must
    /// scan for the variable-width terminator and decode the payload in the same
    /// pass. This avoids the older buffered path that first scanned for a
    /// terminator and then asked the codec to scan and decode the same bytes
    /// again.
    ///
    /// # Type Parameters
    ///
    /// * `N` - The maximum number of bytes that can belong to the
    ///   variable-width payload.
    /// * `T` - The decoded value type.
    /// * `E` - The decoder-specific error type.
    /// * `F` - The decoder function type.
    /// * `M` - The error mapping function type.
    ///
    /// # Arguments
    ///
    /// * `decode_available` - Function that decodes from currently buffered
    ///   bytes. It returns `Ok(None)` when more input is needed and
    ///   `Err((error, consumed))` when invalid bytes should be consumed before
    ///   reporting the mapped error.
    /// * `map_error` - Function that converts decoder errors into
    ///   [`std::io::Error`].
    ///
    /// # Returns
    ///
    /// The decoded value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if EOF is reached before a complete
    /// or maximum-width payload is buffered. Returns any non-interrupted I/O
    /// error produced by the wrapped reader while refilling the buffer. Returns
    /// `map_error(error)` when `decode_available` rejects the buffered payload.
    #[inline(always)]
    pub(crate) fn read_variable_decoded<const N: usize, T, E, F, M>(
        &mut self,
        mut decode_available: F,
        map_error: M,
    ) -> Result<T>
    where
        F: FnMut(&[u8], usize, usize) -> std::result::Result<Option<(T, usize)>, (E, usize)>,
        M: FnOnce(E) -> Error,
    {
        debug_assert!(
            N <= self.buffer.len(),
            "variable payload length exceeds buffer capacity"
        );
        loop {
            let available = self.available().min(N);
            if available > 0 {
                let index = self.position;
                match decode_available(&self.buffer, index, available) {
                    Ok(Some((value, consumed))) => {
                        debug_assert!(consumed > 0, "decoded payload consumed no bytes");
                        debug_assert!(consumed <= available, "decoded beyond available bytes");
                        self.position = index + consumed;
                        return Ok(value);
                    }
                    Ok(None) => {
                        debug_assert!(available < N, "decoder must reject maximum-width unterminated payload");
                    }
                    Err((error, consumed)) => {
                        debug_assert!(consumed > 0, "invalid payload consumed no bytes");
                        debug_assert!(consumed <= available, "invalid payload exceeded buffer");
                        self.position = index + consumed;
                        return Err(map_error(error));
                    }
                }
            }
            if self.available() == 0 {
                self.discard_buffer();
            } else if self.tail_capacity() == 0 {
                self.backshift();
            }
            if !self.read_more()? {
                self.position = self.limit;
                return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
            }
        }
    }

    /// Reads raw bytes through the internal buffer.
    ///
    /// If the internal buffer is empty and `output` is at least as large as the
    /// buffer, the read is delegated directly to the wrapped reader to avoid an
    /// unnecessary copy. Otherwise, bytes are served from the internal buffer.
    ///
    /// # Arguments
    ///
    /// * `output` - Destination slice that receives the bytes read.
    ///
    /// # Returns
    ///
    /// The number of bytes written to `output`. A return value of `0` means that
    /// `output` was empty or EOF was reached before any bytes were read.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced by the wrapped reader. Interrupted reads
    /// are retried when the method refills the internal buffer through
    /// [`Self::read_more`]; direct delegated reads follow the wrapped reader's
    /// own [`Read::read`] behavior.
    pub(crate) fn read_raw(&mut self, output: &mut [u8]) -> Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.available() == 0 {
            self.discard_buffer();
            if output.len() >= self.buffer.len() {
                return self.inner.read(output);
            }
            if !self.read_more()? {
                return Ok(0);
            }
        }
        let count = output.len().min(self.available());
        output[..count].copy_from_slice(&self.buffer[self.position..self.position + count]);
        self.position += count;
        Ok(count)
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
    pub(crate) fn seek_raw(&mut self, position: SeekFrom) -> Result<u64>
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

impl<R> Read for BufferedInput<R>
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
    #[inline]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        self.read_raw(output)
    }
}
