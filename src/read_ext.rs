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

use crate::copy::copy_limited;

/// Default stack buffer size used by discard operations.
const DISCARD_BUFFER_SIZE: usize = 8 * 1024;

/// Default stack buffer size used by bounded read operations.
const READ_TO_END_BUFFER_SIZE: usize = 8 * 1024;

/// Extension methods for [`Read`] values.
///
/// `ReadExt` fills small semantic gaps in the standard [`Read`] trait while
/// keeping the same blocking and error model. The methods are implemented for
/// every type that implements [`Read`], including `dyn Read` trait objects.
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
    fn copy_to_limited(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64>;

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
}

impl<T> ReadExt for T
where
    T: Read,
{
    #[inline]
    fn read_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        read_exact_or_eof_from(self, buffer)
    }

    #[inline]
    fn discard_exact_or_eof(&mut self, bytes: u64) -> Result<u64> {
        discard_exact_or_eof_from(self, bytes)
    }

    #[inline]
    fn copy_to(&mut self, writer: &mut dyn Write) -> Result<u64> {
        copy_all(self, writer)
    }

    #[inline]
    fn copy_to_limited(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
        copy_limited(self, writer, max_bytes)
    }

    #[inline]
    fn read_to_end_limited(&mut self, max_len: usize) -> Result<Vec<u8>> {
        read_to_end_limited_from(self, max_len)
    }
}

impl ReadExt for dyn Read + '_ {
    #[inline]
    fn read_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        read_exact_or_eof_from(self, buffer)
    }

    #[inline]
    fn discard_exact_or_eof(&mut self, bytes: u64) -> Result<u64> {
        discard_exact_or_eof_from(self, bytes)
    }

    #[inline]
    fn copy_to(&mut self, writer: &mut dyn Write) -> Result<u64> {
        copy_all(self, writer)
    }

    #[inline]
    fn copy_to_limited(&mut self, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
        copy_limited(self, writer, max_bytes)
    }

    #[inline]
    fn read_to_end_limited(&mut self, max_len: usize) -> Result<Vec<u8>> {
        read_to_end_limited_from(self, max_len)
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
pub(crate) fn read_exact_or_eof_from(reader: &mut dyn Read, buffer: &mut [u8]) -> Result<usize> {
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
pub(crate) fn discard_exact_or_eof_from(reader: &mut dyn Read, bytes: u64) -> Result<u64> {
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
fn read_to_end_limited_from(reader: &mut dyn Read, max_len: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; READ_TO_END_BUFFER_SIZE];
    loop {
        let remaining = max_len.saturating_sub(output.len());
        let requested = remaining.saturating_add(1).min(READ_TO_END_BUFFER_SIZE);
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => return Ok(output),
            Ok(count) if count <= remaining => output.extend_from_slice(&buffer[..count]),
            Ok(_) => {
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
