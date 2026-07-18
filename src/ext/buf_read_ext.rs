// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    BufRead,
    Error,
    ErrorKind,
    Result,
};

use crate::ext::internal::read_ext_impl;
use crate::util::{
    allocation_error,
    try_reserve_string,
    try_reserve_vec,
};

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
    /// `max_len`. If the limit is exceeded, no vector is returned and the
    /// reader may still consume bytes while detecting the violation.
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
    fn read_until_limited(
        &mut self,
        delimiter: u8,
        max_len: usize,
    ) -> Result<Vec<u8>>;

    /// Reads bytes through `delimiter` into `output` while enforcing `max_len`.
    ///
    /// This method appends at most `max_len` bytes from the current reader
    /// position to `output`. The delimiter is included when it is found. If the
    /// limit is exceeded, `output` is truncated back to its original length.
    /// The reader may still consume bytes while detecting the violation.
    ///
    /// # Parameters
    /// - `delimiter`: Delimiter byte to search for.
    /// - `output`: Destination vector to append to.
    /// - `max_len`: Maximum accepted result length, including the delimiter.
    ///
    /// # Returns
    /// Number of bytes appended to `output`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when more than `max_len` bytes are
    /// required before reaching `delimiter` or EOF. Returns the first I/O error
    /// reported by the underlying reader.
    fn read_until_limited_into(
        &mut self,
        delimiter: u8,
        output: &mut Vec<u8>,
        max_len: usize,
    ) -> Result<usize>;

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

    /// Reads one UTF-8 line into `output` while enforcing `max_len`.
    ///
    /// This method reads at most `max_len` bytes, validates the line as UTF-8,
    /// and appends it to `output`. If the line is oversized or invalid UTF-8,
    /// `output` is truncated back to its original length. Oversized input may
    /// still consume bytes from the reader while detecting the limit violation.
    ///
    /// # Parameters
    /// - `output`: Destination string to append to.
    /// - `max_len`: Maximum accepted line length in bytes, including `\n`.
    ///
    /// # Returns
    /// Number of bytes appended to `output`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the line exceeds `max_len` or is
    /// not valid UTF-8. Returns the first I/O error reported by the underlying
    /// reader.
    fn read_line_limited_into(
        &mut self,
        output: &mut String,
        max_len: usize,
    ) -> Result<usize>;

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
    fn discard_until_limited(
        &mut self,
        delimiter: u8,
        max_len: usize,
    ) -> Result<usize>;
}

impl<T> BufReadExt for T
where
    T: BufRead + ?Sized,
{
    #[inline]
    fn read_until_limited(
        &mut self,
        delimiter: u8,
        max_len: usize,
    ) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        try_reserve_vec(&mut output, max_len.min(8192))
            .map_err(allocation_error)?;
        read_ext_impl::read_until_limited_into(
            self,
            delimiter,
            &mut output,
            max_len,
        )?;
        Ok(output)
    }

    #[inline]
    fn read_until_limited_into(
        &mut self,
        delimiter: u8,
        output: &mut Vec<u8>,
        max_len: usize,
    ) -> Result<usize> {
        read_ext_impl::read_until_limited_into(self, delimiter, output, max_len)
    }

    #[inline]
    fn read_line_limited(&mut self, max_len: usize) -> Result<String> {
        let mut output = String::new();
        self.read_line_limited_into(&mut output, max_len)?;
        Ok(output)
    }

    fn read_line_limited_into(
        &mut self,
        output: &mut String,
        max_len: usize,
    ) -> Result<usize> {
        let original_len = output.len();
        let mut bytes = Vec::new();
        try_reserve_vec(&mut bytes, max_len.min(8192))
            .map_err(allocation_error)?;
        let result = (|| {
            let count = read_ext_impl::read_until_limited_into(
                self, b'\n', &mut bytes, max_len,
            )?;
            let line = String::from_utf8(bytes).map_err(|error| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("limited line is not valid UTF-8: {error}"),
                )
            })?;
            try_reserve_string(output, line.len()).map_err(allocation_error)?;
            output.push_str(&line);
            Ok(count)
        })();
        if result.is_err() {
            output.truncate(original_len);
        }
        result
    }

    fn discard_until_limited(
        &mut self,
        delimiter: u8,
        max_len: usize,
    ) -> Result<usize> {
        let mut discarded = 0;
        loop {
            let available = self.fill_buf()?;
            if available.is_empty() {
                return Ok(discarded);
            }

            let delimiter_position =
                available.iter().position(|byte| *byte == delimiter);
            let requested = delimiter_position
                .map_or(available.len(), |position| position + 1);
            let remaining = max_len.saturating_sub(discarded);
            if requested > remaining {
                if remaining > 0 {
                    self.consume(remaining);
                }
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "input exceeds maximum length of {max_len} bytes before delimiter {delimiter}"
                    ),
                ));
            }

            self.consume(requested);
            discarded += requested;
            if delimiter_position.is_some() {
                return Ok(discarded);
            }
        }
    }
}
