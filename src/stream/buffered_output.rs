/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Result, Seek, SeekFrom, Write};

use crate::stream::buffered_input::{DEFAULT_BUFFER_CAPACITY, MIN_CODEC_BUFFER_CAPACITY};

/// Buffered output core shared by codec-oriented writers.
pub(crate) struct BufferedOutput<W> {
    inner: W,
    buffer: Vec<u8>,
    length: usize,
}

impl<W> BufferedOutput<W> {
    /// Creates a buffered output core with the default capacity.
    #[inline]
    pub(crate) fn new(inner: W) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered output core with at least the requested capacity.
    #[inline]
    pub(crate) fn with_capacity(inner: W, capacity: usize) -> Self {
        let capacity = capacity.max(MIN_CODEC_BUFFER_CAPACITY);
        Self {
            inner,
            buffer: vec![0; capacity],
            length: 0,
        }
    }

    /// Returns a shared reference to the wrapped writer.
    #[inline]
    pub(crate) const fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped writer.
    #[inline]
    pub(crate) fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W> BufferedOutput<W>
where
    W: Write,
{
    /// Consumes this buffered output after flushing pending bytes.
    #[inline]
    pub(crate) fn into_inner(mut self) -> Result<W> {
        self.flush_buffer()?;
        Ok(self.inner)
    }

    /// Ensures that `count` bytes can be written into the internal buffer.
    #[inline]
    fn ensure_space(&mut self, count: usize) -> Result<()> {
        debug_assert!(
            count <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
        if self.buffer.len() - self.length < count {
            self.flush_buffer()?;
        }
        Ok(())
    }

    /// Encodes one value directly into the internal buffer.
    #[inline]
    pub(crate) fn write_encoded<T, F>(&mut self, max_len: usize, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], usize, T) -> usize,
    {
        self.ensure_space(max_len)?;
        let start = self.length;
        let written = encode(&mut self.buffer, start, value);
        debug_assert!(written <= max_len, "codec wrote more bytes than declared");
        self.length += written;
        Ok(())
    }

    /// Writes raw bytes through the internal buffer.
    pub(crate) fn write_all_buffered(&mut self, mut input: &[u8]) -> Result<()> {
        while !input.is_empty() {
            if self.length == 0 && input.len() >= self.buffer.len() {
                self.inner.write_all(input)?;
                return Ok(());
            }
            if self.length == self.buffer.len() {
                self.flush_buffer()?;
            }
            let space = self.buffer.len() - self.length;
            let count = input.len().min(space);
            self.buffer[self.length..self.length + count].copy_from_slice(&input[..count]);
            self.length += count;
            input = &input[count..];
        }
        Ok(())
    }

    /// Flushes buffered bytes to the wrapped writer.
    pub(crate) fn flush_buffer(&mut self) -> Result<()> {
        if self.length > 0 {
            self.inner.write_all(&self.buffer[..self.length])?;
            self.length = 0;
        }
        Ok(())
    }

    /// Flushes buffered bytes and then flushes the wrapped writer.
    pub(crate) fn flush_all(&mut self) -> Result<()> {
        self.flush_buffer()?;
        self.inner.flush()
    }

    /// Writes raw bytes and reports the full input length on success.
    pub(crate) fn write_raw(&mut self, input: &[u8]) -> Result<usize> {
        self.write_all_buffered(input)?;
        Ok(input.len())
    }

    /// Flushes pending bytes before seeking the wrapped writer.
    pub(crate) fn seek_raw(&mut self, position: SeekFrom) -> Result<u64>
    where
        W: Seek,
    {
        self.flush_buffer()?;
        self.inner.seek(position)
    }
}
