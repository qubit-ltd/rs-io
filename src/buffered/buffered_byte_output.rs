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
    Seek,
    SeekFrom,
    Write,
};

use crate::Buffer;
use crate::WriteExt;
use crate::buffered::buffered_byte_input::DEFAULT_BUFFER_CAPACITY;

/// Buffered byte output over a wrapped writer.
///
/// This type keeps a fixed-size byte buffer in front of an underlying writer so
/// small byte writes can be accumulated before they are written to the I/O
/// target. Large writes may bypass the buffer after pending buffered bytes
/// have been flushed.
#[derive(Debug)]
pub struct BufferedByteOutput<W> {
    inner: W,
    buffer: Buffer<u8>,
}

impl<W> BufferedByteOutput<W> {
    /// Creates a buffered byte output with the default capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The writer that receives bytes when the internal buffer is
    ///   flushed.
    ///
    /// # Returns
    ///
    /// A new buffered byte output using `DEFAULT_BUFFER_CAPACITY`.
    #[inline(always)]
    pub fn new(inner: W) -> Self {
        Self::with_capacity(inner, DEFAULT_BUFFER_CAPACITY)
    }

    /// Creates a buffered byte output with at least the requested capacity.
    ///
    /// # Parameters
    ///
    /// * `inner` - The writer that receives bytes when the internal buffer is
    ///   flushed.
    /// * `capacity` - The requested internal buffer capacity in bytes.
    ///
    /// # Returns
    ///
    /// A new buffered byte output whose actual buffer capacity is
    /// `capacity.max(1)`.
    #[inline(always)]
    pub fn with_capacity(inner: W, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }

    /// Returns a shared reference to the wrapped writer.
    ///
    /// # Returns
    ///
    /// An immutable reference to the underlying writer.  Pending bytes may
    /// still be present in the internal buffer and are not flushed by this
    /// method.
    #[inline(always)]
    pub const fn inner(&self) -> &W {
        &self.inner
    }

    /// Returns an exclusive reference to the wrapped writer.
    ///
    /// Pending bytes may still be present in the internal buffer and are not
    /// flushed by this method.
    ///
    /// # Returns
    ///
    /// A mutable reference to the underlying writer.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the unused capacity in the internal buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes that can still be appended to the internal buffer
    /// before it must be flushed.
    #[inline(always)]
    pub fn spare_capacity(&self) -> usize {
        self.buffer.spare_capacity()
    }

    /// Returns the unused portion of the internal buffer.
    ///
    /// Callers may write initialized bytes into the returned slice and then
    /// call [`Self::advance`] with the number of bytes written.
    ///
    /// # Returns
    ///
    /// A mutable slice over the spare buffer capacity.
    #[inline(always)]
    #[must_use]
    pub fn spare_buffer_mut(&mut self) -> &mut [u8] {
        let limit = self.buffer.limit();
        &mut self.buffer.data_mut()[limit..]
    }

    /// Marks `count` bytes from [`Self::spare_buffer_mut`] as written.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of bytes initialized by the caller.
    ///
    /// # Panics
    ///
    /// Panics when `count` exceeds [`Self::spare_capacity`].
    #[inline(always)]
    pub fn advance(&mut self, count: usize) {
        assert!(
            count <= self.spare_capacity(),
            "cannot advance beyond spare output buffer"
        );
        // SAFETY: The assertion proves that `count` is within spare capacity.
        unsafe {
            self.buffer.advance_unchecked(count);
        }
    }

    /// Writes bytes into the internal buffer without checking spare capacity.
    ///
    /// # Parameters
    ///
    /// * `input` - The source bytes.
    /// * `input_index` - The starting index in `input`.
    /// * `count` - The number of bytes to copy.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `input_index..input_index + count` is valid
    /// in `input`, that `count <= self.spare_capacity()`, and that the copied
    /// source range does not overlap with the destination range in the internal
    /// buffer.
    #[inline(always)]
    unsafe fn write_to_buffer_unchecked(
        &mut self,
        input: &[u8],
        input_index: usize,
        count: usize,
    ) {
        // SAFETY: The caller upholds `Buffer::copy_from_unchecked` range and
        // non-overlap requirements.
        unsafe {
            self.buffer.copy_from_unchecked(input, input_index, count);
        }
    }
}

impl<W> BufferedByteOutput<W>
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
    #[inline(always)]
    pub fn into_inner(mut self) -> Result<W> {
        self.flush_buffer().map(|()| self.inner)
    }

    /// Ensures that at least `count` bytes are available in the spare buffer.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of spare bytes required.
    ///
    /// # Errors
    ///
    /// Returns any non-interrupted I/O error produced while flushing buffered
    /// bytes. Returns [`ErrorKind::InvalidInput`] if `count` exceeds the buffer
    /// capacity.
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

    /// Writes all bytes through the internal buffer.
    ///
    /// Small inputs are appended to the internal buffer.  Inputs that do not
    /// fit may flush the buffer first, and inputs at least as large as the
    /// buffer may be written directly to the wrapped writer.
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
    fn write_all_buffered(&mut self, input: &[u8]) -> Result<()> {
        if input.len() < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer_unchecked(input, 0, input.len());
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
        if input.len() >= self.buffer.capacity() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.inner.write_all_unchecked(input, 0, input.len()) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer_unchecked(input, 0, input.len());
            }
            Ok(())
        }
    }

    /// Handles slow-path raw writes for [`Write::write`] semantics.
    ///
    /// The method preserves `Write::write` behavior: it may accept fewer bytes
    /// than the input length when the write is delegated directly to the
    /// wrapped writer.
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
        if input.len() >= self.buffer.capacity() {
            // SAFETY: The range covers the full source slice.
            unsafe { self.inner.write_unchecked(input, 0, input.len()) }
        } else {
            // SAFETY: After the optional flush, any input smaller than the
            // buffer capacity fits in the empty or sufficiently spare buffer.
            unsafe {
                self.write_to_buffer_unchecked(input, 0, input.len());
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
    pub fn flush_buffer(&mut self) -> Result<()> {
        while !self.buffer.is_empty() {
            let position = self.buffer.position();
            let available = self.buffer.available();
            // SAFETY: `position..position + available` is the current readable
            // range maintained by `Buffer`.
            match unsafe {
                self.inner.write_unchecked(
                    self.buffer.data(),
                    position,
                    available,
                )
            } {
                Ok(0) => {
                    self.buffer.compact();
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write buffered data",
                    ));
                }
                Ok(written) => {
                    // SAFETY: `write_unchecked` returns a count in
                    // `0..=available`.
                    unsafe {
                        self.buffer.consume_unchecked(written);
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
    #[inline(always)]
    fn flush_all(&mut self) -> Result<()> {
        self.flush_buffer().and_then(|()| self.inner.flush())
    }

    /// Writes bytes from the input slice and reports the accepted byte count.
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
    fn write_from(&mut self, input: &[u8]) -> Result<usize> {
        if input.len() < self.spare_capacity() {
            // SAFETY: The branch proves that the input fits in spare capacity.
            unsafe {
                self.write_to_buffer_unchecked(input, 0, input.len());
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
    #[inline(always)]
    fn flush_then_seek(&mut self, position: SeekFrom) -> Result<u64>
    where
        W: Seek,
    {
        self.flush_buffer().and_then(|()| self.inner.seek(position))
    }
}

impl<W> Write for BufferedByteOutput<W>
where
    W: Write,
{
    /// Writes bytes through the internal buffer.
    #[inline(always)]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.write_from(buffer)
    }

    /// Writes all bytes through the internal buffer.
    #[inline(always)]
    fn write_all(&mut self, buffer: &[u8]) -> Result<()> {
        self.write_all_buffered(buffer)
    }

    /// Flushes the internal buffer and then the wrapped writer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.flush_all()
    }
}

impl<W> Seek for BufferedByteOutput<W>
where
    W: Write + Seek,
{
    /// Flushes pending bytes before seeking the wrapped writer.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.flush_then_seek(position)
    }
}
