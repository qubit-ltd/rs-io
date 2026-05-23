/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Error, ErrorKind, Result, Seek, SeekFrom, Write};
use std::ptr;

use crate::WriteExt;
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

    /// Returns the unused capacity in the internal buffer.
    #[inline]
    fn spare_capacity(&self) -> usize {
        self.buffer.len() - self.length
    }

    /// Writes bytes into the internal buffer without checking spare capacity.
    #[inline]
    unsafe fn write_to_buffer_unchecked(&mut self, input: &[u8]) {
        debug_assert!(input.len() <= self.spare_capacity());
        let old_len = self.length;
        let input_len = input.len();
        // SAFETY: The caller guarantees that the destination range is within
        // the initialized internal buffer and does not overlap the source.
        unsafe {
            let destination = self.buffer.as_mut_ptr().add(old_len);
            ptr::copy_nonoverlapping(input.as_ptr(), destination, input_len);
        }
        self.length = old_len + input_len;
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
    #[cold]
    #[inline(never)]
    fn ensure_space_slow(&mut self, count: usize) -> Result<()> {
        debug_assert!(
            count <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
        self.flush_buffer()?;
        Ok(())
    }

    /// Encodes one value directly into the internal buffer.
    #[inline]
    pub(crate) fn write_encoded<T, F>(&mut self, max_len: usize, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], usize, T) -> usize,
    {
        debug_assert!(
            max_len <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
        if self.spare_capacity() < max_len {
            self.ensure_space_slow(max_len)?;
        }
        let start = self.length;
        let written = encode(&mut self.buffer, start, value);
        debug_assert!(written <= max_len, "codec wrote more bytes than declared");
        // Keep this assignment based on the saved cursor instead of writing
        // `self.length += written`. The encoder receives `&mut self.buffer`;
        // on the fixed-width hot path, recomputing the cursor from
        // `self.length` after that mutable borrow makes LLVM reload more
        // state and measurably slows binary writes. Using `start + written`
        // states the actual invariant directly: the cursor advances from the
        // position that was checked before the codec wrote into the buffer.
        self.length = start + written;
        Ok(())
    }

    /// Encodes one fixed-width value directly into the internal buffer.
    #[inline]
    pub(crate) fn write_fixed<const N: usize, T, F>(&mut self, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], usize, T),
    {
        debug_assert!(
            N <= self.buffer.len(),
            "requested range exceeds buffer capacity"
        );
        if self.spare_capacity() < N {
            self.ensure_space_slow(N)?;
        }
        let start = self.length;
        encode(&mut self.buffer, start, value);
        self.length = start + N;
        Ok(())
    }

    /// Writes raw bytes through the internal buffer.
    #[inline]
    pub(crate) fn write_all_buffered(&mut self, input: &[u8]) -> Result<()> {
        if input.len() < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer_unchecked(input);
            }
            Ok(())
        } else {
            self.write_all_cold(input)
        }
    }

    /// Handles slow-path raw writes that must flush or bypass the buffer.
    #[cold]
    #[inline(never)]
    fn write_all_cold(&mut self, input: &[u8]) -> Result<()> {
        if input.len() > self.spare_capacity() {
            self.flush_buffer()?;
        }
        if input.len() >= self.buffer.len() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.inner.write_all_unchecked(input, 0, input.len()) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer_unchecked(input);
            }
            Ok(())
        }
    }

    /// Handles slow-path raw writes for [`Write::write`] semantics.
    #[cold]
    #[inline(never)]
    fn write_cold(&mut self, input: &[u8]) -> Result<usize> {
        if input.len() > self.spare_capacity() {
            self.flush_buffer()?;
        }
        if input.len() >= self.buffer.len() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.inner.write_unchecked(input, 0, input.len()) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer_unchecked(input);
            }
            Ok(input.len())
        }
    }

    /// Flushes buffered bytes to the wrapped writer.
    pub(crate) fn flush_buffer(&mut self) -> Result<()> {
        struct BufferGuard<'a> {
            buffer: &'a mut [u8],
            length: &'a mut usize,
            written: usize,
        }

        impl BufferGuard<'_> {
            /// Returns the number of not-yet-written buffered bytes.
            #[inline]
            fn remaining_len(&self) -> usize {
                *self.length - self.written
            }

            /// Records that `count` more bytes have been written.
            #[inline]
            fn consume(&mut self, count: usize) {
                self.written += count;
            }

            /// Returns whether all buffered bytes have been written.
            #[inline]
            fn done(&self) -> bool {
                self.written >= *self.length
            }
        }

        impl Drop for BufferGuard<'_> {
            fn drop(&mut self) {
                if self.written == 0 {
                    return;
                }
                let remaining = *self.length - self.written;
                if remaining > 0 {
                    self.buffer.copy_within(self.written..*self.length, 0);
                }
                *self.length = remaining;
            }
        }

        let mut guard = BufferGuard {
            buffer: &mut self.buffer,
            length: &mut self.length,
            written: 0,
        };
        while !guard.done() {
            let remaining_len = guard.remaining_len();
            // SAFETY: `written..length` is maintained as a valid range inside
            // the initialized output buffer.
            match unsafe {
                self.inner
                    .write_unchecked(guard.buffer, guard.written, remaining_len)
            } {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write buffered data",
                    ));
                }
                Ok(written) => guard.consume(written),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Flushes buffered bytes and then flushes the wrapped writer.
    pub(crate) fn flush_all(&mut self) -> Result<()> {
        self.flush_buffer()?;
        self.inner.flush()
    }

    /// Writes raw bytes and reports the full input length on success.
    #[inline]
    pub(crate) fn write_raw(&mut self, input: &[u8]) -> Result<usize> {
        if input.len() < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer_unchecked(input);
            }
            Ok(input.len())
        } else {
            self.write_cold(input)
        }
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
