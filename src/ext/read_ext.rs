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
    Write,
    copy as copy_all,
};
use std::string::FromUtf8Error;

use crate::{
    copy_at_most,
    copy_to_end_limited as copy_to_end_limited_fn,
};

/// Default stack buffer size used by discard operations.
const DISCARD_BUFFER_SIZE: usize = 8 * 1024;

/// Default stack buffer size used by bounded read operations.
const READ_TO_END_BUFFER_SIZE: usize = 8 * 1024;

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
    fn read_exact_vec_limited(&mut self, len: usize, max_len: usize) -> Result<Vec<u8>>;

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
    fn copy_to_at_most(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64>;

    /// Copies the remaining input if its total length is at most `max_bytes`.
    ///
    /// This method copies from the current reader position until EOF. If EOF is
    /// not reached within `max_bytes` bytes, it returns
    /// [`ErrorKind::InvalidData`]. Detecting oversized input consumes one
    /// excess byte from this reader; that excess byte is not written to
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
    fn copy_to_end_limited(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64>;

    /// Reads the remaining bytes into a vector with a maximum accepted length.
    ///
    /// This method consumes bytes from the current reader position until EOF is
    /// reached. If the stream contains more than `max_len` bytes, it returns
    /// [`ErrorKind::InvalidData`] after detecting the first excess byte.
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
    /// byte. In that case, the accepted prefix may already have been appended
    /// to `output`, and one excess byte may have been consumed from the reader.
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
    fn read_to_end_limited_into(&mut self, output: &mut Vec<u8>, max_len: usize) -> Result<usize>;

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
    /// `output`. If the input is oversized or invalid UTF-8, `output` is left
    /// unchanged. Oversized input may still consume up to `max_len + 1` bytes
    /// from the reader while detecting the limit violation.
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
    fn read_to_string_limited_into(&mut self, output: &mut String, max_len: usize)
    -> Result<usize>;
}

impl<T> ReadExt for T
where
    T: Read + ?Sized,
{
    #[inline]
    fn read_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        read_exact_or_eof_impl(&mut reader, buffer)
    }

    #[inline]
    fn read_exact_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut reader = self;
        read_exact_array_impl::<N>(&mut reader)
    }

    #[inline]
    fn read_exact_vec_limited(&mut self, len: usize, max_len: usize) -> Result<Vec<u8>> {
        let mut reader = self;
        read_exact_vec_limited_impl(&mut reader, len, max_len)
    }

    #[inline]
    fn read_exact_vec_limited_into(
        &mut self,
        output: &mut Vec<u8>,
        len: usize,
        max_len: usize,
    ) -> Result<()> {
        let mut reader = self;
        read_exact_vec_limited_into_impl(&mut reader, output, len, max_len)
    }

    #[inline]
    fn discard_exact_or_eof(&mut self, bytes: u64) -> Result<u64> {
        let mut reader = self;
        discard_exact_or_eof_impl(&mut reader, bytes)
    }

    #[inline]
    fn copy_to(&mut self, writer: &mut dyn Write) -> Result<u64> {
        let mut reader = self;
        copy_to_impl(&mut reader, writer)
    }

    #[inline]
    fn copy_to_at_most(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
        let mut reader = self;
        copy_at_most(&mut reader, writer, max_bytes)
    }

    #[inline]
    fn copy_to_end_limited(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
        let mut reader = self;
        copy_to_end_limited_fn(&mut reader, writer, max_bytes)
    }

    #[inline]
    fn read_to_end_limited(&mut self, max_len: usize) -> Result<Vec<u8>> {
        let mut reader = self;
        read_to_end_limited_impl(&mut reader, max_len)
    }

    #[inline]
    fn read_to_end_limited_into(&mut self, output: &mut Vec<u8>, max_len: usize) -> Result<usize> {
        let mut reader = self;
        read_to_end_limited_into_impl(&mut reader, output, max_len)
    }

    #[inline]
    fn read_to_string_limited(&mut self, max_len: usize) -> Result<String> {
        let mut reader = self;
        read_to_string_limited_impl(&mut reader, max_len)
    }

    #[inline]
    fn read_to_string_limited_into(
        &mut self,
        output: &mut String,
        max_len: usize,
    ) -> Result<usize> {
        let mut reader = self;
        read_to_string_limited_into_impl(&mut reader, output, max_len)
    }
}

/// Reads from `reader` until `buffer` is full or EOF is reached.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `buffer`: Destination buffer to fill.
///
/// # Returns
/// The number of bytes written into `buffer`.
///
/// # Errors
/// Returns the first non-interrupted read error reported by `reader`.
fn read_exact_or_eof_impl(reader: &mut dyn Read, buffer: &mut [u8]) -> Result<usize> {
    let mut total = 0;
    while total < buffer.len() {
        match reader.read(&mut buffer[total..]) {
            Ok(0) => break,
            Ok(count) => total += count,
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

/// Reads exactly `N` bytes from `reader` into an array.
///
/// # Parameters
/// - `reader`: Source reader.
///
/// # Returns
/// A stack-allocated array containing exactly `N` bytes.
///
/// # Errors
/// Returns the error reported by [`Read::read_exact`].
fn read_exact_array_impl<const N: usize>(reader: &mut dyn Read) -> Result<[u8; N]> {
    let mut buffer = [0; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Reads exactly `len` bytes from `reader` when `len` is within `max_len`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `len`: Exact number of bytes to read.
/// - `max_len`: Maximum accepted exact read length.
///
/// # Returns
/// A vector containing exactly `len` bytes.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when `len > max_len`. Returns the error
/// reported by [`Read::read_exact`] for read failures.
fn read_exact_vec_limited_impl(
    reader: &mut dyn Read,
    len: usize,
    max_len: usize,
) -> Result<Vec<u8>> {
    validate_exact_read_len(len, max_len)?;
    let mut output = Vec::with_capacity(len);
    read_exact_vec_limited_into_impl(reader, &mut output, len, max_len)?;
    Ok(output)
}

/// Reads exactly `len` bytes from `reader` and appends them to `output`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `output`: Destination vector to append to.
/// - `len`: Exact number of bytes to read.
/// - `max_len`: Maximum accepted exact read length.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when `len > max_len` before reading and
/// leaves `output` unchanged. Returns the error reported by
/// [`Read::read_exact`] for read failures and truncates `output` back to its
/// original length.
fn read_exact_vec_limited_into_impl(
    reader: &mut dyn Read,
    output: &mut Vec<u8>,
    len: usize,
    max_len: usize,
) -> Result<()> {
    validate_exact_read_len(len, max_len)?;
    let original_len = output.len();
    output.resize(original_len + len, 0);
    match reader.read_exact(&mut output[original_len..]) {
        Ok(()) => Ok(()),
        Err(error) => {
            output.truncate(original_len);
            Err(error)
        }
    }
}

/// Validates that an exact read length is within the configured maximum.
///
/// # Parameters
/// - `len`: Exact number of bytes requested by the caller.
/// - `max_len`: Maximum accepted exact read length.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when `len > max_len`.
fn validate_exact_read_len(len: usize, max_len: usize) -> Result<()> {
    if len > max_len {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("requested length {len} exceeds maximum length {max_len}"),
        ));
    }
    Ok(())
}

/// Discards up to `bytes` bytes from `reader`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `bytes`: Maximum number of bytes to discard.
///
/// # Returns
/// The number of bytes actually discarded.
///
/// # Errors
/// Returns the first non-interrupted read error reported by `reader`.
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

/// Reads all remaining bytes from `reader` when the result fits `max_len`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `max_len`: Maximum accepted result length.
///
/// # Returns
/// A vector containing all remaining bytes.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] after detecting that the input contains
/// more than `max_len` bytes. Returns the first non-interrupted read error
/// reported by `reader`.
fn read_to_end_limited_impl(reader: &mut dyn Read, max_len: usize) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(max_len.min(READ_TO_END_BUFFER_SIZE));
    read_to_end_limited_into_impl(reader, &mut output, max_len)?;
    Ok(output)
}

/// Reads all remaining bytes from `reader` into `output` when the input fits.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `output`: Destination vector to append to.
/// - `max_len`: Maximum accepted input length in bytes.
///
/// # Returns
/// The number of bytes appended to `output`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] after detecting that the input contains
/// more than `max_len` bytes. Returns the first non-interrupted read error
/// reported by `reader`.
fn read_to_end_limited_into_impl(
    reader: &mut dyn Read,
    output: &mut Vec<u8>,
    max_len: usize,
) -> Result<usize> {
    let mut buffer = [0; READ_TO_END_BUFFER_SIZE];
    let mut appended = 0;
    loop {
        let remaining = max_len.saturating_sub(appended);
        let requested = remaining.saturating_add(1).min(READ_TO_END_BUFFER_SIZE);
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => return Ok(appended),
            Ok(count) if count <= remaining => {
                output.extend_from_slice(&buffer[..count]);
                appended += count;
            }
            Ok(_) => {
                if remaining > 0 {
                    output.extend_from_slice(&buffer[..remaining]);
                }
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("input exceeds maximum length of {max_len} bytes"),
                ));
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
}

/// Reads all remaining bytes from `reader` as UTF-8 when the input fits `max_len`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `max_len`: Maximum accepted input length in bytes.
///
/// # Returns
/// Decoded UTF-8 string.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when the input is oversized or is not
/// valid UTF-8. Returns the first non-interrupted read error reported by
/// `reader`.
fn read_to_string_limited_impl(reader: &mut dyn Read, max_len: usize) -> Result<String> {
    let bytes = read_to_end_limited_impl(reader, max_len)?;
    String::from_utf8(bytes).map_err(invalid_utf8_error)
}

/// Reads all remaining UTF-8 text from `reader` into `output`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `output`: Destination string to append to.
/// - `max_len`: Maximum accepted input length in bytes.
///
/// # Returns
/// The number of bytes appended to `output`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when the input is oversized or is not
/// valid UTF-8. Returns the first non-interrupted read error reported by
/// `reader`.
fn read_to_string_limited_into_impl(
    reader: &mut dyn Read,
    output: &mut String,
    max_len: usize,
) -> Result<usize> {
    let bytes = read_to_end_limited_impl(reader, max_len)?;
    let text = String::from_utf8(bytes).map_err(invalid_utf8_error)?;
    let count = text.len();
    output.push_str(&text);
    Ok(count)
}

/// Copies all remaining bytes from `reader` into `writer`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `writer`: Destination writer.
///
/// # Returns
/// The number of bytes copied.
///
/// # Errors
/// Returns the first read or write error reported by the underlying streams.
fn copy_to_impl(reader: &mut dyn Read, writer: &mut dyn Write) -> Result<u64> {
    copy_all(reader, writer)
}

/// Converts an invalid UTF-8 read result into an I/O error.
///
/// # Parameters
/// - `error`: UTF-8 conversion error.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error containing the UTF-8 error context.
fn invalid_utf8_error(error: FromUtf8Error) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("limited input is not valid UTF-8: {error}"),
    )
}
