// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal implementation helpers for [`Read`] and [`BufRead`] extension
//! methods.
//!
//! This module provides free functions that back
//! [`crate::std_io::ext::ReadExt`] and [`crate::std_io::ext::BufReadExt`]. The
//! functions are public within the crate so sibling modules and tests can call
//! them, but they are not re-exported from the crate root and are intended for
//! internal use only.
use std::io::BufRead;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;
use std::string::FromUtf8Error;

use crate::util::allocation_error;
use crate::util::try_reserve_vec;

/// Default stack buffer size used by bounded read operations.
pub(crate) const READ_TO_END_BUFFER_SIZE: usize = 8 * 1024;

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
pub(crate) fn read_exact_or_eof(
    reader: &mut dyn Read,
    buffer: &mut [u8],
) -> Result<usize> {
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

/// Reads exactly `len` bytes from `reader` and appends them to `output`.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `output`: Destination vector to append to.
/// - `len`: Exact number of bytes to read.
/// - `max_len`: Maximum accepted exact read length.
///
/// # Returns
///
/// Returns `Ok(())` after exactly `len` bytes are appended.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when `len > max_len` before reading and
/// leaves `output` unchanged. Returns [`ErrorKind::InvalidInput`] when
/// `output.len() + len` overflows, or [`ErrorKind::OutOfMemory`] when `output`
/// cannot reserve the appended bytes. Returns the error reported by
/// [`Read::read_exact`] for read failures and truncates `output` back to its
/// original length.
pub(crate) fn read_exact_vec_limited_into(
    reader: &mut dyn Read,
    output: &mut Vec<u8>,
    len: usize,
    max_len: usize,
) -> Result<()> {
    validate_exact_read_len(len, max_len)?;
    let original_len = output.len();
    let new_len = match original_len.checked_add(len) {
        Some(value) => value,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("length {original_len} plus {len} overflows usize"),
            ));
        }
    };
    try_reserve_vec(output, len).map_err(allocation_error)?;
    output.resize(new_len, 0);
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
/// # Returns
///
/// Returns `Ok(())` when `len <= max_len`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when `len > max_len`.
#[inline]
pub(crate) fn validate_exact_read_len(
    len: usize,
    max_len: usize,
) -> Result<()> {
    if len > max_len {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("requested length {len} exceeds maximum length {max_len}"),
        ));
    }
    Ok(())
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
/// more than `max_len` bytes. Returns [`ErrorKind::OutOfMemory`] when the
/// result vector cannot grow, or the first non-interrupted read error reported
/// by `reader`. No vector is returned on failure.
#[inline]
pub(crate) fn read_to_end_limited(
    reader: &mut dyn Read,
    max_len: usize,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    try_reserve_vec(&mut output, max_len.min(READ_TO_END_BUFFER_SIZE))
        .map_err(allocation_error)?;
    read_to_end_limited_into(reader, &mut output, max_len)?;
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
/// more than `max_len` bytes. Returns [`ErrorKind::OutOfMemory`] when `output`
/// cannot grow, or the first non-interrupted read error reported by `reader`.
/// `output` is restored to its original length on failure.
pub(crate) fn read_to_end_limited_into(
    reader: &mut dyn Read,
    output: &mut Vec<u8>,
    max_len: usize,
) -> Result<usize> {
    let original_len = output.len();
    let mut buffer = [0; READ_TO_END_BUFFER_SIZE];
    let mut appended = 0;
    loop {
        let remaining = max_len.saturating_sub(appended);
        // The extra byte is intentional, even when `max_len == 0`, so EOF can
        // be distinguished from input that exceeds the configured limit.
        let requested =
            remaining.saturating_add(1).min(READ_TO_END_BUFFER_SIZE);
        match reader.read(&mut buffer[..requested]) {
            Ok(count) => {
                if count == 0 {
                    return Ok(appended);
                } else if count <= remaining {
                    if let Err(error) = try_reserve_vec(output, count) {
                        output.truncate(original_len);
                        return Err(allocation_error(error));
                    }
                    output.extend_from_slice(&buffer[..count]);
                    appended += count;
                } else {
                    output.truncate(original_len);
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "input exceeds maximum length of {max_len} bytes"
                        ),
                    ));
                }
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                output.truncate(original_len);
                return Err(error);
            }
        }
    }
}

/// Converts an invalid UTF-8 read result into an I/O error.
///
/// # Parameters
/// - `error`: UTF-8 conversion error.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error containing the UTF-8 error context.
#[inline(always)]
#[must_use]
pub(crate) fn invalid_utf8_error(error: FromUtf8Error) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("limited input is not valid UTF-8: {error}"),
    )
}

/// Reads bytes through `delimiter` into `output` with a maximum result size.
///
/// # Type Parameters
///
/// - `T`: Buffered reader type.
///
/// # Parameters
/// - `reader`: Buffered source reader.
/// - `delimiter`: Delimiter byte to search for.
/// - `output`: Destination vector to append to.
/// - `max_len`: Maximum accepted result length.
///
/// # Returns
/// Number of bytes appended to `output`.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when the limit is exceeded,
/// [`ErrorKind::OutOfMemory`] when `output` cannot grow, or an I/O error from
/// `reader`.
#[inline]
pub(crate) fn read_until_limited_into<T>(
    reader: &mut T,
    delimiter: u8,
    output: &mut Vec<u8>,
    max_len: usize,
) -> Result<usize>
where
    T: BufRead + ?Sized,
{
    let original_len = output.len();
    match read_until_limited_into_inner(reader, delimiter, output, max_len) {
        Ok(count) => Ok(count),
        Err(error) => {
            output.truncate(original_len);
            Err(error)
        }
    }
}

/// Appends bytes through `delimiter` without rollback on failure.
///
/// # Type Parameters
///
/// - `T`: Buffered reader type.
///
/// # Parameters
///
/// - `reader`: Buffered source reader.
/// - `delimiter`: Delimiter byte to search for.
/// - `output`: Destination vector.
/// - `max_len`: Maximum accepted byte count.
///
/// # Returns
///
/// Returns the number of appended bytes.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if reaching the delimiter or EOF requires
/// more than `max_len` bytes, [`ErrorKind::OutOfMemory`] if `output` cannot
/// grow, or the first error reported by `reader`. Bytes appended before an
/// error remain in `output`.
fn read_until_limited_into_inner<T>(
    reader: &mut T,
    delimiter: u8,
    output: &mut Vec<u8>,
    max_len: usize,
) -> Result<usize>
where
    T: BufRead + ?Sized,
{
    let mut appended = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(appended);
        }

        let delimiter_position =
            available.iter().position(|byte| *byte == delimiter);
        let requested =
            delimiter_position.map_or(available.len(), |position| position + 1);
        let remaining = max_len.saturating_sub(appended);
        if requested > remaining {
            if remaining > 0 {
                reader.consume(remaining);
            }
            return Err(limit_exceeded_error(max_len, delimiter));
        }

        try_reserve_vec(output, requested).map_err(allocation_error)?;
        output.extend_from_slice(&available[..requested]);
        reader.consume(requested);
        appended += requested;
        if delimiter_position.is_some() {
            return Ok(appended);
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
#[inline]
#[must_use]
fn limit_exceeded_error(max_len: usize, delimiter: u8) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!(
            "input exceeds maximum length of {max_len} bytes before delimiter {delimiter}"
        ),
    )
}
