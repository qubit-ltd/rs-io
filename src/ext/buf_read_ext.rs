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
    BufRead,
    Error,
    ErrorKind,
    Result,
};
use std::string::FromUtf8Error;

/// Extension methods for [`BufRead`] values.
///
/// `BufReadExt` provides bounded delimiter-oriented reads. These helpers are
/// useful for line-based and delimiter-based formats where accepting unbounded
/// input would make parsers vulnerable to excessive memory use.
pub trait BufReadExt: BufRead {
    /// Reads bytes through `delimiter` while enforcing `max_len`.
    ///
    /// The returned vector includes the delimiter when it is found. EOF before
    /// the delimiter is accepted as long as the accumulated bytes do not exceed
    /// `max_len`.
    ///
    /// # Parameters
    /// - `delimiter`: Delimiter byte to search for.
    /// - `max_len`: Maximum accepted result length, including the delimiter.
    ///
    /// # Returns
    /// Bytes read from the stream.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when more than `max_len` bytes are
    /// required before reaching `delimiter` or EOF. Returns the first I/O error
    /// reported by the underlying reader.
    fn read_until_limited(&mut self, delimiter: u8, max_len: usize) -> Result<Vec<u8>>;

    /// Reads one UTF-8 line while enforcing `max_len`.
    ///
    /// The returned string includes the trailing `\n` when it is present. EOF
    /// before a newline is accepted as long as the accumulated bytes do not
    /// exceed `max_len`.
    ///
    /// # Parameters
    /// - `max_len`: Maximum accepted line length in bytes, including `\n`.
    ///
    /// # Returns
    /// The decoded UTF-8 line.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the line exceeds `max_len` or is
    /// not valid UTF-8. Returns the first I/O error reported by the underlying
    /// reader.
    fn read_line_limited(&mut self, max_len: usize) -> Result<String>;

    /// Discards bytes through `delimiter` while enforcing `max_len`.
    ///
    /// The delimiter is consumed when it is found. EOF before the delimiter is
    /// accepted as long as no more than `max_len` bytes are consumed.
    ///
    /// # Parameters
    /// - `delimiter`: Delimiter byte to search for.
    /// - `max_len`: Maximum number of bytes to discard, including the
    ///   delimiter.
    ///
    /// # Returns
    /// Number of bytes discarded.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when more than `max_len` bytes are
    /// required before reaching `delimiter` or EOF. Returns the first I/O error
    /// reported by the underlying reader.
    fn discard_until_limited(&mut self, delimiter: u8, max_len: usize) -> Result<usize>;
}

impl<T> BufReadExt for T
where
    T: BufRead + ?Sized,
{
    #[inline]
    fn read_until_limited(&mut self, delimiter: u8, max_len: usize) -> Result<Vec<u8>> {
        read_until_limited_impl(self, delimiter, max_len)
    }

    #[inline]
    fn read_line_limited(&mut self, max_len: usize) -> Result<String> {
        read_line_limited_impl(self, max_len)
    }

    #[inline]
    fn discard_until_limited(&mut self, delimiter: u8, max_len: usize) -> Result<usize> {
        discard_until_limited_impl(self, delimiter, max_len)
    }
}

/// Reads bytes through `delimiter` with a maximum result size.
///
/// # Parameters
/// - `reader`: Buffered source reader.
/// - `delimiter`: Delimiter byte to search for.
/// - `max_len`: Maximum accepted result length.
///
/// # Returns
/// Bytes read from the stream.
///
/// # Errors
/// Returns an invalid-data error when the limit is exceeded, or an I/O error
/// from `reader`.
fn read_until_limited_impl<T>(reader: &mut T, delimiter: u8, max_len: usize) -> Result<Vec<u8>>
where
    T: BufRead + ?Sized,
{
    let mut output = Vec::with_capacity(max_len.min(8192));
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(output);
        }

        let delimiter_position = available.iter().position(|byte| *byte == delimiter);
        let requested = delimiter_position.map_or(available.len(), |position| position + 1);
        let remaining = max_len.saturating_sub(output.len());
        if requested > remaining {
            if remaining > 0 {
                output.extend_from_slice(&available[..remaining]);
                reader.consume(remaining);
            }
            return Err(limit_exceeded_error(max_len, delimiter));
        }

        output.extend_from_slice(&available[..requested]);
        reader.consume(requested);
        if delimiter_position.is_some() {
            return Ok(output);
        }
    }
}

/// Reads one UTF-8 line with a maximum byte length.
///
/// # Parameters
/// - `reader`: Buffered source reader.
/// - `max_len`: Maximum accepted line length in bytes.
///
/// # Returns
/// Decoded line.
///
/// # Errors
/// Returns an invalid-data error when the line exceeds the limit or is not
/// valid UTF-8, or an I/O error from `reader`.
fn read_line_limited_impl<T>(reader: &mut T, max_len: usize) -> Result<String>
where
    T: BufRead + ?Sized,
{
    let bytes = read_until_limited_impl(reader, b'\n', max_len)?;
    String::from_utf8(bytes).map_err(invalid_utf8_error)
}

/// Discards bytes through `delimiter` with a maximum consumed size.
///
/// # Parameters
/// - `reader`: Buffered source reader.
/// - `delimiter`: Delimiter byte to search for.
/// - `max_len`: Maximum accepted discard length.
///
/// # Returns
/// Number of discarded bytes.
///
/// # Errors
/// Returns an invalid-data error when the limit is exceeded, or an I/O error
/// from `reader`.
fn discard_until_limited_impl<T>(reader: &mut T, delimiter: u8, max_len: usize) -> Result<usize>
where
    T: BufRead + ?Sized,
{
    let mut discarded = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(discarded);
        }

        let delimiter_position = available.iter().position(|byte| *byte == delimiter);
        let requested = delimiter_position.map_or(available.len(), |position| position + 1);
        let remaining = max_len.saturating_sub(discarded);
        if requested > remaining {
            if remaining > 0 {
                reader.consume(remaining);
            }
            return Err(limit_exceeded_error(max_len, delimiter));
        }

        reader.consume(requested);
        discarded += requested;
        if delimiter_position.is_some() {
            return Ok(discarded);
        }
    }
}

/// Builds an invalid-data error for delimiter reads that exceed their limit.
///
/// # Parameters
/// - `max_len`: Maximum accepted byte length.
/// - `delimiter`: Delimiter byte searched by the caller.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error.
fn limit_exceeded_error(max_len: usize, delimiter: u8) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("input exceeds maximum length of {max_len} bytes before delimiter {delimiter}"),
    )
}

/// Converts an invalid UTF-8 line error into an I/O error.
///
/// # Parameters
/// - `error`: UTF-8 conversion error.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error containing the UTF-8 error context.
fn invalid_utf8_error(error: FromUtf8Error) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("limited line is not valid UTF-8: {error}"),
    )
}
