/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};

use crate::ReadExt;

/// Default capacity used by buffered codec readers and writers.
pub(crate) const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Minimum capacity required by the largest scalar codec payload.
pub(crate) const MIN_CODEC_BUFFER_CAPACITY: usize = 19;

/// Buffered input core shared by codec-oriented readers.
pub(crate) struct BufferedInput<R> {
    inner: R,
    buffer: Vec<u8>,
    position: usize,
    limit: usize,
}

impl<R> BufferedInput<R> {
    /// Creates a buffered input core with the default capacity.
    #[inline]
    pub(crate) fn new(inner: R) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered input core with at least the requested capacity.
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

    /// Returns a shared reference to the wrapped reader.
    #[inline]
    pub(crate) const fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped reader.
    #[inline]
    pub(crate) fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this buffered input and returns the wrapped reader.
    #[inline]
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the number of unread bytes currently buffered.
    #[inline]
    fn available(&self) -> usize {
        self.limit - self.position
    }

    /// Returns the unused capacity at the end of the buffer.
    #[inline]
    fn tail_capacity(&self) -> usize {
        self.buffer.len() - self.limit
    }

    /// Invalidates all buffered bytes.
    #[inline]
    fn discard_buffer(&mut self) {
        self.position = 0;
        self.limit = 0;
    }

    /// Moves unread bytes to the front of the buffer.
    #[inline]
    fn backshift(&mut self) {
        if self.position == 0 {
            return;
        }
        if self.position == self.limit {
            self.discard_buffer();
            return;
        }
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
    fn read_more(&mut self) -> Result<bool> {
        let count = self.tail_capacity();
        debug_assert!(count > 0, "buffer has no tail capacity");
        loop {
            // SAFETY: `limit` is always within `buffer`, and `count` is the
            // remaining capacity from `limit` to the end of `buffer`.
            match unsafe {
                self.inner
                    .read_unchecked(&mut self.buffer, self.limit, count)
            } {
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
    fn ensure_available_slow(&mut self, count: usize) -> Result<()> {
        debug_assert!(
            count <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
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
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
        }
        Ok(())
    }

    /// Reads one fixed-width value directly from the internal buffer.
    #[inline]
    pub(crate) fn read_fixed<const N: usize, T, F>(&mut self, decode: F) -> Result<T>
    where
        F: FnOnce(&[u8], usize) -> T,
    {
        debug_assert!(
            N <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
        if self.available() < N {
            self.ensure_available_slow(N)?;
        }
        let index = self.position;
        let value = decode(&self.buffer, index);
        self.position = index + N;
        Ok(value)
    }

    /// Reads one variable-width value directly from the internal buffer.
    pub(crate) fn read_variable<T, E, F, M>(
        &mut self,
        max_len: usize,
        decode: F,
        map_error: M,
    ) -> Result<T>
    where
        F: FnOnce(&[u8], usize) -> std::result::Result<(T, usize), E>,
        M: FnOnce(E) -> Error,
    {
        let decode_len = match self.variable_payload_len(max_len) {
            Some(len) => len,
            None => self.ensure_variable_payload_slow(max_len)?,
        };
        let index = self.position;
        match decode(&self.buffer, index) {
            Ok((value, consumed)) => {
                self.position += consumed;
                Ok(value)
            }
            Err(error) => {
                self.position += decode_len;
                Err(map_error(error))
            }
        }
    }

    /// Finds the available length of a terminated or max-width variable payload.
    #[inline]
    fn variable_payload_len(&self, max_len: usize) -> Option<usize> {
        let available = self.available();
        let scan_len = available.min(max_len);
        for offset in 0..scan_len {
            let byte = self.buffer[self.position + offset];
            if byte & 0x80 == 0 {
                return Some(offset + 1);
            }
        }
        if available >= max_len {
            Some(max_len)
        } else {
            None
        }
    }

    /// Ensures that a terminated or max-width variable payload is buffered.
    fn ensure_variable_payload_slow(&mut self, max_len: usize) -> Result<usize> {
        debug_assert!(
            max_len <= self.buffer.len(),
            "variable payload length exceeds buffer capacity"
        );
        loop {
            if let Some(len) = self.variable_payload_len(max_len) {
                return Ok(len);
            }
            if self.available() == 0 {
                self.discard_buffer();
            } else if self.tail_capacity() == 0 {
                self.backshift();
            }
            if !self.read_more()? {
                self.position = self.limit;
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
        }
    }

    /// Reads raw bytes through the internal buffer.
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

    /// Seeks the wrapped reader after discarding buffered bytes.
    pub(crate) fn seek_raw(&mut self, position: SeekFrom) -> Result<u64>
    where
        R: Seek,
    {
        let unread = self.available() as i64;
        self.position = 0;
        self.limit = 0;
        match position {
            SeekFrom::Current(offset) => self.inner.seek(SeekFrom::Current(offset - unread)),
            other => self.inner.seek(other),
        }
    }
}

impl<R> Read for BufferedInput<R>
where
    R: Read,
{
    /// Reads bytes through the internal buffer.
    #[inline]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        self.read_raw(output)
    }
}
