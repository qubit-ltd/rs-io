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
    Result,
    Seek,
    SeekFrom,
    Write,
};
use std::ptr;

use crate::WriteExt;
use crate::stream::buffered_input::{
    DEFAULT_BUFFER_CAPACITY,
    MIN_CODEC_BUFFER_CAPACITY,
};

/// Buffered output core shared by codec-oriented writers.
///
/// This type keeps a fixed-size byte buffer in front of an underlying writer so
/// small encoded values can be accumulated before they are written to the I/O
/// target.  Large raw writes may bypass the buffer after pending buffered bytes
/// have been flushed.
pub(crate) struct BufferedOutput<W> {
    inner: W,
    buffer: Vec<u8>,
    length: usize,
}

impl<W> BufferedOutput<W> {
    /// Creates a buffered output core with the default capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The writer that receives bytes when the internal buffer is
    ///   flushed.
    ///
    /// # Returns
    ///
    /// A new buffered output wrapper using [`DEFAULT_BUFFER_CAPACITY`], adjusted
    /// by [`with_capacity`](Self::with_capacity) to satisfy the minimum codec
    /// buffer size.
    #[inline]
    pub(crate) fn new(inner: W) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered output core with at least the requested capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The writer that receives bytes when the internal buffer is
    ///   flushed.
    /// * `capacity` - The requested internal buffer capacity in bytes.
    ///
    /// # Returns
    ///
    /// A new buffered output wrapper whose actual buffer capacity is
    /// `capacity.max(MIN_CODEC_BUFFER_CAPACITY)`.
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
    ///
    /// # Returns
    ///
    /// An immutable reference to the underlying writer.  Pending bytes may still
    /// be present in the internal buffer and are not flushed by this method.
    #[inline]
    pub(crate) const fn inner(&self) -> &W {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// A mutable reference to the underlying writer.  Pending bytes may still be
    /// present in the internal buffer and are not flushed by this method.
    #[inline]
    pub(crate) fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the unused capacity in the internal buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes that can still be appended to the internal buffer
    /// before it must be flushed.
    #[inline]
    fn spare_capacity(&self) -> usize {
        self.buffer.len() - self.length
    }

    /// Writes bytes into the internal buffer without checking spare capacity.
    ///
    /// # Parameters
    ///
    /// * `input` - The bytes to append to the internal buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `input.len() <= self.spare_capacity()` and
    /// that `input` does not overlap with the destination range in the internal
    /// buffer.  Violating either requirement may cause memory corruption.
    #[inline]
    unsafe fn write_to_buffer_unchecked(&mut self, input: &[u8]) {
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
    ///
    /// # Returns
    ///
    /// The wrapped writer after all pending buffered bytes have been written.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes.  Also returns [`ErrorKind::WriteZero`] if the wrapped writer
    /// reports that zero bytes were written before the buffer is drained.
    #[inline]
    pub(crate) fn into_inner(mut self) -> Result<W> {
        self.flush_buffer().map(|()| self.inner)
    }

    /// Ensures that `count` bytes can be written into the internal buffer.
    ///
    /// If there is not enough spare capacity, the currently buffered bytes are
    /// flushed to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `count` - The number of bytes that must fit in the internal buffer.
    ///
    /// # Returns
    ///
    /// `Ok(())` once at least `count` bytes can be written into the buffer.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes.  Also returns [`ErrorKind::WriteZero`] if the wrapped writer
    /// reports that zero bytes were written before the buffer is drained.
    #[cold]
    #[inline(never)]
    fn ensure_space_slow(&mut self, _count: usize) -> Result<()> {
        self.flush_buffer()
    }

    /// Encodes one value directly into the internal buffer.
    ///
    /// The encoder receives the entire internal buffer, the current write
    /// position, and the value to encode.  It must write no more than `max_len`
    /// bytes starting at that position and return the number of bytes written.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The value type accepted by the encoder.
    /// * `F` - The encoder callback type.
    ///
    /// # Parameters
    ///
    /// * `max_len` - The maximum number of bytes that the encoder may write.
    /// * `value` - The value passed to the encoder.
    /// * `encode` - The callback that serializes `value` into the internal
    ///   buffer.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the encoded bytes have been reserved in the buffer and the
    /// buffered length has been advanced.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes to make room for `max_len`.  Also returns [`ErrorKind::WriteZero`]
    /// if the wrapped writer reports that zero bytes were written before the
    /// buffer is drained.
    #[inline]
    pub(crate) fn write_encoded<T, F>(&mut self, max_len: usize, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], usize, T) -> usize,
    {
        if self.spare_capacity() < max_len {
            self.ensure_space_slow(max_len)?;
        }
        let start = self.length;
        let written = encode(&mut self.buffer, start, value);
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
    ///
    /// The encoder receives the entire internal buffer, the current write
    /// position, and the value to encode.  It must write exactly `N` bytes
    /// starting at that position.
    ///
    /// # Type Parameters
    ///
    /// * `N` - The fixed encoded width, in bytes.
    /// * `T` - The value type accepted by the encoder.
    /// * `F` - The encoder callback type.
    ///
    /// # Parameters
    ///
    /// * `value` - The value passed to the encoder.
    /// * `encode` - The callback that serializes `value` into the internal
    ///   buffer.
    ///
    /// # Returns
    ///
    /// `Ok(())` after `N` bytes have been reserved in the buffer and the
    /// buffered length has been advanced.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes to make room for `N`.  Also returns [`ErrorKind::WriteZero`] if the
    /// wrapped writer reports that zero bytes were written before the buffer is
    /// drained.
    #[inline]
    pub(crate) fn write_fixed<const N: usize, T, F>(&mut self, value: T, encode: F) -> Result<()>
    where
        F: FnOnce(&mut [u8], usize, T),
    {
        if self.spare_capacity() < N {
            self.ensure_space_slow(N)?;
        }
        let start = self.length;
        encode(&mut self.buffer, start, value);
        self.length = start + N;
        Ok(())
    }

    /// Writes raw bytes through the internal buffer.
    ///
    /// Small inputs are appended to the internal buffer.  Inputs that do not fit
    /// may flush the buffer first, and inputs at least as large as the buffer may
    /// be written directly to the wrapped writer.
    ///
    /// # Parameters
    ///
    /// * `input` - The bytes to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all bytes from `input` have been accepted.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending bytes or writing a
    /// large input directly to the wrapped writer.  Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero bytes were
    /// written before the buffer is drained.
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
    ///
    /// # Parameters
    ///
    /// * `input` - The bytes to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all bytes from `input` have been accepted either by the
    /// buffer or by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending bytes or writing a
    /// large input directly to the wrapped writer.  Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero bytes were
    /// written before the buffer is drained.
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
    ///
    /// The method preserves `Write::write` behavior: it may accept fewer bytes
    /// than the input length when the write is delegated directly to the wrapped
    /// writer.
    ///
    /// # Parameters
    ///
    /// * `input` - The bytes to write after the fast path determined that they
    ///   do not fit comfortably in the current spare buffer capacity.
    ///
    /// # Returns
    ///
    /// The number of bytes accepted.  Buffered writes return `input.len()`;
    /// direct writes return the byte count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending bytes or writing a
    /// large input directly to the wrapped writer.  Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero bytes were
    /// written before the buffer is drained.
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
    ///
    /// The method retries interrupted writes.  If an error occurs after some
    /// bytes have been written, the already-written bytes are removed from the
    /// front of the buffer and the unwritten suffix is kept for a later retry.
    ///
    /// # Returns
    ///
    /// `Ok(())` once all currently buffered bytes have been written to the
    /// wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced by the wrapped writer.
    /// Returns [`ErrorKind::WriteZero`] if the writer reports a zero-length
    /// write before all buffered bytes are drained.
    pub(crate) fn flush_buffer(&mut self) -> Result<()> {
        struct BufferGuard<'a> {
            buffer: &'a mut [u8],
            length: &'a mut usize,
            written: usize,
        }

        impl BufferGuard<'_> {
            /// Returns the number of not-yet-written buffered bytes.
            ///
            /// # Returns
            ///
            /// The remaining byte count between the guard's current write
            /// cursor and the buffered length captured by the guard.
            #[inline]
            fn remaining_len(&self) -> usize {
                *self.length - self.written
            }

            /// Records that `count` more bytes have been written.
            ///
            /// # Parameters
            ///
            /// * `count` - The number of newly written bytes to add to the
            ///   guard's progress counter.
            #[inline]
            fn consume(&mut self, count: usize) {
                self.written += count;
            }

            /// Returns whether all buffered bytes have been written.
            ///
            /// # Returns
            ///
            /// `true` when the guard has recorded at least the buffered length
            /// as written; otherwise `false`.
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
            match unsafe { self.inner.write_unchecked(guard.buffer, guard.written, remaining_len) } {
                Ok(0) => {
                    return Err(Error::new(ErrorKind::WriteZero, "failed to write buffered data"));
                }
                Ok(written) => guard.consume(written),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Flushes buffered bytes and then flushes the wrapped writer.
    ///
    /// # Returns
    ///
    /// `Ok(())` once pending buffered bytes have been written and the wrapped
    /// writer's own flush operation succeeds.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes, [`ErrorKind::WriteZero`] if the wrapped writer cannot make
    /// progress while draining the buffer, or any error returned by
    /// [`Write::flush`] on the wrapped writer.
    pub(crate) fn flush_all(&mut self) -> Result<()> {
        self.flush_buffer().and_then(|()| self.inner.flush())
    }

    /// Writes raw bytes and reports the accepted byte count.
    ///
    /// This is the buffered implementation for [`Write::write`]-style callers.
    /// Small inputs are appended to the buffer and reported as fully accepted;
    /// large inputs may be delegated to the wrapped writer after pending bytes
    /// are flushed.
    ///
    /// # Parameters
    ///
    /// * `input` - The bytes to write.
    ///
    /// # Returns
    ///
    /// The number of bytes accepted.  Buffered writes return `input.len()`;
    /// direct writes return the byte count reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while flushing pending bytes or writing a
    /// large input directly to the wrapped writer.  Flush failures include
    /// [`ErrorKind::WriteZero`] if the writer reports that zero bytes were
    /// written before the buffer is drained.
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
    ///
    /// # Parameters
    ///
    /// * `position` - The target seek position passed to the wrapped writer.
    ///
    /// # Returns
    ///
    /// The new stream position reported by the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes, [`ErrorKind::WriteZero`] if the wrapped writer cannot make
    /// progress while draining the buffer, or any error returned by
    /// [`Seek::seek`] on the wrapped writer.
    pub(crate) fn seek_raw(&mut self, position: SeekFrom) -> Result<u64>
    where
        W: Seek,
    {
        self.flush_buffer().and_then(|()| self.inner.seek(position))
    }
}
