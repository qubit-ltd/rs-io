// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    ErrorKind,
    Read,
    Result,
    Write,
    copy as copy_all,
};

use crate::Streams;
use crate::ext::internal::read_ext_impl;
use crate::util::{
    UncheckedSlice,
    allocation_error,
    try_reserve_string,
};

/// Default stack buffer size used by discard operations.
const DISCARD_BUFFER_SIZE: usize = 8 * 1024;

/// Extension methods for [`Read`] values.
///
/// `ReadExt` fills small semantic gaps in the standard [`Read`] trait while
/// keeping the same blocking and error model. The methods are implemented for
/// every type that implements [`Read`], including `dyn Read` trait objects.
///
/// # Examples
/// ```
/// use qubit_io::ReadExt;
/// use std::io::Cursor;
///
/// let mut input = Cursor::new(b"abcdef".to_vec());
/// let header = input.read_exact_array::<2>()?;
/// let payload = input.read_exact_vec_limited(4, 16)?;
///
/// assert_eq!(*b"ab", header);
/// assert_eq!(b"cdef", payload.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait ReadExt: Read {
    /// Reads bytes into a range of `buffer` without checking the range bounds
    /// in release builds.
    ///
    /// This method delegates to [`Read::read`] after creating the target slice
    /// with raw pointer arithmetic. It performs at most one read operation and
    /// returns the number of bytes read, keeping the same short-read and error
    /// behavior as [`Read::read`].
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Maximum number of bytes to read.
    ///
    /// # Returns
    /// The number of bytes written into `buffer[start_index..start_index +
    /// count]`. The value is in `0..=count`.
    ///
    /// # Errors
    /// Returns the error reported by [`Read::read`].
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    unsafe fn read_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Reads exactly `count` bytes into a range of `buffer` without checking
    /// the range bounds in release builds.
    ///
    /// This method delegates to [`Read::read_exact`] after creating the target
    /// slice with raw pointer arithmetic. It keeps the same blocking and error
    /// behavior as [`Read::read_exact`].
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Number of bytes to read.
    ///
    /// # Errors
    /// Returns the error reported by [`Read::read_exact`], including
    /// [`ErrorKind::UnexpectedEof`] when EOF is reached before `count` bytes
    /// are read.
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    unsafe fn read_exact_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<()>;

    /// Reads bytes into a range of `buffer` until that range is full or EOF is
    /// reached, without checking the range bounds in release builds.
    ///
    /// This method has the same EOF and retry behavior as
    /// [`ReadExt::read_exact_or_eof`], but writes into
    /// `buffer[start_index..start_index + count]` using raw pointer
    /// arithmetic.
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Number of bytes to try to read.
    ///
    /// # Returns
    /// The number of bytes written into `buffer[start_index..start_index +
    /// count]`. The value is in `0..=count`.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] error reported by the
    /// underlying reader. Interrupted reads are retried.
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    unsafe fn read_exact_or_eof_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Reads bytes until `buffer` is full or EOF is reached.
    ///
    /// This method differs from [`Read::read_exact`] by treating EOF as a
    /// successful partial result. It keeps retrying short reads until the
    /// caller-provided buffer is full, EOF is reached, or a non-interrupted
    /// I/O error occurs.
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer to fill.
    ///
    /// # Returns
    /// The number of bytes written into `buffer`. The value is in
    /// `0..=buffer.len()`.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] error reported by the
    /// underlying reader. Interrupted reads are retried.
    fn read_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Reads exactly `N` bytes into a stack-allocated array.
    ///
    /// This method uses [`Read::read_exact`] and therefore requires the reader
    /// to provide exactly `N` bytes before EOF.
    ///
    /// # Returns
    /// An array containing exactly `N` bytes read from this reader.
    ///
    /// # Errors
    /// Returns the error reported by [`Read::read_exact`], including
    /// [`ErrorKind::UnexpectedEof`] when EOF is reached before the array is
    /// full.
    fn read_exact_array<const N: usize>(&mut self) -> Result<[u8; N]>;

    /// Reads exactly `len` bytes into a new vector after checking a limit.
    ///
    /// If `len` is greater than `max_len`, this method returns
    /// [`ErrorKind::InvalidData`] before reading any bytes.
    ///
    /// # Parameters
    /// - `len`: Exact number of bytes to read.
    /// - `max_len`: Maximum accepted exact read length.
    ///
    /// # Returns
    /// A vector containing exactly `len` bytes.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when `len > max_len`. Returns the
    /// error reported by [`Read::read_exact`], including
    /// [`ErrorKind::UnexpectedEof`] when EOF is reached before `len` bytes are
    /// read.
    fn read_exact_vec_limited(
        &mut self,
        len: usize,
        max_len: usize,
    ) -> Result<Vec<u8>>;

    /// Reads exactly `len` bytes and appends them to `output`.
    ///
    /// If `len` is greater than `max_len`, this method returns
    /// [`ErrorKind::InvalidData`] before reading any bytes and leaves `output`
    /// unchanged. On a read error, `output` is truncated back to its original
    /// length. The underlying reader may still have consumed bytes before the
    /// error because [`Read`] does not provide rollback.
    ///
    /// # Parameters
    /// - `output`: Destination vector to append to.
    /// - `len`: Exact number of bytes to read.
    /// - `max_len`: Maximum accepted exact read length.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when `len > max_len`. Returns the
    /// error reported by [`Read::read_exact`], including
    /// [`ErrorKind::UnexpectedEof`] when EOF is reached before `len` bytes are
    /// read.
    fn read_exact_vec_limited_into(
        &mut self,
        output: &mut Vec<u8>,
        len: usize,
        max_len: usize,
    ) -> Result<()>;

    /// Discards up to `bytes` bytes from this reader.
    ///
    /// The method repeatedly reads into an internal stack buffer until the
    /// requested number of bytes has been consumed or EOF is reached. It does
    /// not allocate and does not require seeking support.
    ///
    /// # Parameters
    /// - `bytes`: Maximum number of bytes to discard.
    ///
    /// # Returns
    /// The number of bytes actually discarded. The value may be smaller than
    /// `bytes` when EOF is reached first.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] error reported by the
    /// underlying reader. Interrupted reads are retried.
    fn discard_exact_or_eof(&mut self, bytes: u64) -> Result<u64>;

    /// Copies all remaining bytes from this reader into `writer`.
    ///
    /// This method is a method-style wrapper around [`std::io::copy`]. It
    /// copies from the current reader position until EOF and does not close or
    /// flush either stream.
    ///
    /// # Parameters
    /// - `writer`: Destination writer.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns the first read or write error reported by the underlying
    /// streams, using the same error behavior as [`std::io::copy`].
    fn copy_to(&mut self, writer: &mut dyn Write) -> Result<u64>;

    /// Copies at most `max_bytes` bytes from this reader into `writer`.
    ///
    /// This method stops successfully when either EOF is reached or
    /// `max_bytes` bytes have been copied. It does not close or flush either
    /// stream.
    ///
    /// # Parameters
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum number of bytes to copy.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] read error or write
    /// error reported by the underlying streams. Interrupted reads are retried.
    fn copy_to_at_most(
        &mut self,
        writer: &mut dyn Write,
        max_bytes: u64,
    ) -> Result<u64>;

    /// Copies the remaining input if its total length is at most `max_bytes`.
    ///
    /// This method copies from the current reader position until EOF. If EOF is
    /// not reached within `max_bytes` bytes, it returns
    /// [`ErrorKind::InvalidData`]. Detecting oversized input consumes one
    /// excess byte from this reader; that excess byte is not written to
    /// `writer`.
    ///
    /// Unlike bounded reads into in-memory collections, this method cannot roll
    /// back bytes already accepted by `writer` when the limit is exceeded
    /// because [`Write`] does not provide truncation. On
    /// [`ErrorKind::InvalidData`], up to `max_bytes` bytes may remain in
    /// `writer`.
    ///
    /// # Parameters
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum accepted number of bytes in the remaining input.
    ///
    /// # Returns
    /// The number of bytes copied when EOF is reached within the limit.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the remaining input is longer
    /// than `max_bytes`. Returns the first non-[`ErrorKind::Interrupted`] read
    /// error or write error reported by the underlying streams. Interrupted
    /// reads are retried.
    fn copy_to_end_limited(
        &mut self,
        writer: &mut dyn Write,
        max_bytes: u64,
    ) -> Result<u64>;

    /// Reads the remaining bytes into a vector with a maximum accepted length.
    ///
    /// This method consumes bytes from the current reader position until EOF is
    /// reached. If the stream contains more than `max_len` bytes, it returns
    /// [`ErrorKind::InvalidData`] after detecting the first excess byte. No
    /// vector is returned on failure.
    ///
    /// # Parameters
    /// - `max_len`: Maximum number of bytes accepted in the returned vector.
    ///
    /// # Returns
    /// A vector containing all remaining bytes when the stream length is within
    /// the limit.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the stream contains more than
    /// `max_len` bytes. Returns the first non-[`ErrorKind::Interrupted`] error
    /// reported by the underlying reader; interrupted reads are retried.
    fn read_to_end_limited(&mut self, max_len: usize) -> Result<Vec<u8>>;

    /// Reads the remaining bytes into `output` with a maximum accepted length.
    ///
    /// This method appends at most `max_len` bytes from the current reader
    /// position to `output`. If the stream contains more than `max_len` bytes,
    /// it returns [`ErrorKind::InvalidData`] after detecting the first excess
    /// byte and truncates `output` back to its original length. The underlying
    /// reader may still have consumed bytes before the error because [`Read`]
    /// does not provide rollback.
    ///
    /// # Parameters
    /// - `output`: Destination vector to append to.
    /// - `max_len`: Maximum number of bytes accepted from this reader.
    ///
    /// # Returns
    /// The number of bytes appended to `output`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the stream contains more than
    /// `max_len` bytes. Returns the first non-[`ErrorKind::Interrupted`] error
    /// reported by the underlying reader; interrupted reads are retried.
    fn read_to_end_limited_into(
        &mut self,
        output: &mut Vec<u8>,
        max_len: usize,
    ) -> Result<usize>;

    /// Reads the remaining bytes as UTF-8 text with a maximum accepted length.
    ///
    /// This method has the same size limit and read semantics as
    /// [`ReadExt::read_to_end_limited`], then validates the collected bytes as
    /// UTF-8.
    ///
    /// # Parameters
    /// - `max_len`: Maximum number of bytes accepted before UTF-8 decoding.
    ///
    /// # Returns
    /// The decoded UTF-8 string.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the stream contains more than
    /// `max_len` bytes or when the collected bytes are not valid UTF-8. Returns
    /// the first non-[`ErrorKind::Interrupted`] error reported by the
    /// underlying reader; interrupted reads are retried.
    fn read_to_string_limited(&mut self, max_len: usize) -> Result<String>;

    /// Reads the remaining bytes as UTF-8 text and appends to `output`.
    ///
    /// This method accepts at most `max_len` bytes from the current reader
    /// position, validates them as UTF-8, and appends the decoded text to
    /// `output`. If the input is oversized or invalid UTF-8, `output` is
    /// truncated back to its original length. Oversized input may still consume
    /// up to `max_len + 1` bytes from the reader while detecting the limit
    /// violation.
    ///
    /// # Parameters
    /// - `output`: Destination string to append to.
    /// - `max_len`: Maximum number of bytes accepted before UTF-8 decoding.
    ///
    /// # Returns
    /// The number of bytes appended to `output`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the stream contains more than
    /// `max_len` bytes or when the collected bytes are not valid UTF-8. Returns
    /// the first non-[`ErrorKind::Interrupted`] error reported by the
    /// underlying reader; interrupted reads are retried.
    fn read_to_string_limited_into(
        &mut self,
        output: &mut String,
        max_len: usize,
    ) -> Result<usize>;
}

/// Reads an indexed range to EOF through a type-erased reader.
unsafe fn read_exact_or_eof_unchecked_impl(
    reader: &mut dyn Read,
    buffer: &mut [u8],
    start_index: usize,
    count: usize,
) -> Result<usize> {
    debug_assert!(
        UncheckedSlice::range_fits(buffer.len(), start_index, count),
        "unchecked read range exceeds buffer"
    );
    let mut total = 0;
    while total < count {
        // SAFETY: The caller guarantees that `start_index..start_index + count`
        // is valid for `buffer`; `total < count`, so this suffix is valid.
        let target = unsafe {
            UncheckedSlice::subslice_mut(
                buffer,
                start_index + total,
                count - total,
            )
        };
        match reader.read(target) {
            Ok(0) => break,
            Ok(read) => total += read,
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Ok(total)
}

/// Reads a fixed-size byte array through a type-erased reader.
fn read_exact_array_impl<const N: usize>(
    reader: &mut dyn Read,
) -> Result<[u8; N]> {
    let mut buffer = [0; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Reads an exactly sized bounded vector through a type-erased reader.
fn read_exact_vec_limited_impl(
    reader: &mut dyn Read,
    len: usize,
    max_len: usize,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    read_ext_impl::read_exact_vec_limited_into(
        reader,
        &mut output,
        len,
        max_len,
    )?;
    Ok(output)
}

/// Discards a bounded byte count through a type-erased reader.
fn discard_exact_or_eof_impl(reader: &mut dyn Read, bytes: u64) -> Result<u64> {
    let mut buffer = [0; DISCARD_BUFFER_SIZE];
    let mut remaining = bytes;
    let mut discarded = 0;
    while remaining > 0 {
        let requested = remaining.min(DISCARD_BUFFER_SIZE as u64) as usize;
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => break,
            Ok(count) => {
                let count = count as u64;
                remaining -= count;
                discarded += count;
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Ok(discarded)
}

/// Reads bounded UTF-8 text through a type-erased reader.
fn read_to_string_limited_impl(
    reader: &mut dyn Read,
    max_len: usize,
) -> Result<String> {
    let bytes = read_ext_impl::read_to_end_limited(reader, max_len)?;
    String::from_utf8(bytes).map_err(read_ext_impl::invalid_utf8_error)
}

/// Appends bounded UTF-8 text read through a type-erased reader.
fn read_to_string_limited_into_impl(
    reader: &mut dyn Read,
    output: &mut String,
    max_len: usize,
) -> Result<usize> {
    let bytes = read_ext_impl::read_to_end_limited(reader, max_len)?;
    let text =
        String::from_utf8(bytes).map_err(read_ext_impl::invalid_utf8_error)?;
    let count = text.len();
    try_reserve_string(output, count).map_err(allocation_error)?;
    output.push_str(&text);
    Ok(count)
}

impl<T> ReadExt for T
where
    T: Read + ?Sized,
{
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(buffer.len(), start_index, count),
            "unchecked read range exceeds buffer"
        );
        // SAFETY: The caller guarantees that the requested range is valid for
        // `buffer`.
        let target =
            unsafe { UncheckedSlice::subslice_mut(buffer, start_index, count) };
        self.read(target)
    }

    unsafe fn read_exact_or_eof_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize> {
        let mut reader = self;
        // SAFETY: Forwarded from the trait caller.
        unsafe {
            read_exact_or_eof_unchecked_impl(
                &mut reader,
                buffer,
                start_index,
                count,
            )
        }
    }

    unsafe fn read_exact_unchecked(
        &mut self,
        buffer: &mut [u8],
        start_index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            UncheckedSlice::range_fits(buffer.len(), start_index, count),
            "unchecked read range exceeds buffer"
        );
        // SAFETY: The caller guarantees that the requested range is valid for
        // `buffer`.
        let target =
            unsafe { UncheckedSlice::subslice_mut(buffer, start_index, count) };
        self.read_exact(target)
    }

    fn read_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        read_ext_impl::read_exact_or_eof(&mut reader, buffer)
    }

    #[inline(always)]
    fn read_exact_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut reader = self;
        read_exact_array_impl(&mut reader)
    }

    fn read_exact_vec_limited(
        &mut self,
        len: usize,
        max_len: usize,
    ) -> Result<Vec<u8>> {
        let mut reader = self;
        read_exact_vec_limited_impl(&mut reader, len, max_len)
    }

    #[inline(always)]
    fn read_exact_vec_limited_into(
        &mut self,
        output: &mut Vec<u8>,
        len: usize,
        max_len: usize,
    ) -> Result<()> {
        let mut reader = self;
        read_ext_impl::read_exact_vec_limited_into(
            &mut reader,
            output,
            len,
            max_len,
        )
    }

    fn discard_exact_or_eof(&mut self, bytes: u64) -> Result<u64> {
        let mut reader = self;
        discard_exact_or_eof_impl(&mut reader, bytes)
    }

    #[inline(always)]
    fn copy_to(&mut self, writer: &mut dyn Write) -> Result<u64> {
        copy_all(self, writer)
    }

    #[inline(always)]
    fn copy_to_at_most(
        &mut self,
        writer: &mut dyn Write,
        max_bytes: u64,
    ) -> Result<u64> {
        let mut reader = self;
        Streams::copy_at_most(&mut reader, writer, max_bytes)
    }

    #[inline(always)]
    fn copy_to_end_limited(
        &mut self,
        writer: &mut dyn Write,
        max_bytes: u64,
    ) -> Result<u64> {
        let mut reader = self;
        Streams::copy_to_end_limited(&mut reader, writer, max_bytes)
    }

    #[inline(always)]
    fn read_to_end_limited(&mut self, max_len: usize) -> Result<Vec<u8>> {
        let mut reader = self;
        read_ext_impl::read_to_end_limited(&mut reader, max_len)
    }

    #[inline(always)]
    fn read_to_end_limited_into(
        &mut self,
        output: &mut Vec<u8>,
        max_len: usize,
    ) -> Result<usize> {
        let mut reader = self;
        read_ext_impl::read_to_end_limited_into(&mut reader, output, max_len)
    }

    fn read_to_string_limited(&mut self, max_len: usize) -> Result<String> {
        let mut reader = self;
        read_to_string_limited_impl(&mut reader, max_len)
    }

    fn read_to_string_limited_into(
        &mut self,
        output: &mut String,
        max_len: usize,
    ) -> Result<usize> {
        let mut reader = self;
        read_to_string_limited_into_impl(&mut reader, output, max_len)
    }
}
