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

use crate::traits::validate_read_count;
use crate::util::SliceRange;
use crate::util::UncheckedSlice;
use crate::util::allocation_error;
use crate::util::try_reserve_string;
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
/// Returns [`ErrorKind::InvalidData`] if the reader reports more bytes than
/// requested.
pub(crate) fn read_exact_or_eof(reader: &mut dyn Read, buffer: &mut [u8]) -> Result<usize> {
    let mut total = 0;
    while total < buffer.len() {
        match reader.read(&mut buffer[total..]) {
            Ok(0) => break,
            Ok(count) => {
                validate_read_count(count, buffer.len() - total)?;
                total += count;
            }
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
pub(crate) fn validate_exact_read_len(len: usize, max_len: usize) -> Result<()> {
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
pub(crate) fn read_to_end_limited(reader: &mut dyn Read, max_len: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    try_reserve_vec(&mut output, max_len.min(READ_TO_END_BUFFER_SIZE)).map_err(allocation_error)?;
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
pub(crate) fn read_to_end_limited_into(reader: &mut dyn Read, output: &mut Vec<u8>, max_len: usize) -> Result<usize> {
    let original_len = output.len();
    let mut buffer = [0; READ_TO_END_BUFFER_SIZE];
    let mut appended = 0;
    loop {
        let remaining = max_len.saturating_sub(appended);
        // The extra byte is intentional, even when `max_len == 0`, so EOF can
        // be distinguished from input that exceeds the configured limit.
        let requested = remaining.saturating_add(1).min(READ_TO_END_BUFFER_SIZE);
        match reader.read(&mut buffer[..requested]) {
            Ok(count) => {
                if let Err(error) = validate_read_count(count, requested) {
                    output.truncate(original_len);
                    return Err(error);
                }
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
                        format!("input exceeds maximum length of {max_len} bytes"),
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
/// error remain in `output`. Interrupted errors are retried without resetting
/// accumulated progress; other I/O errors are returned.
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
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            return Ok(appended);
        }

        let remaining = max_len.saturating_sub(appended);
        let search_len = available.len().min(remaining);
        let delimiter_position = available[..search_len].iter().position(|byte| *byte == delimiter);
        let requested = delimiter_position.map_or(available.len(), |position| position + 1);
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
        format!("input exceeds maximum length of {max_len} bytes before delimiter {delimiter}"),
    )
}

/// Default stack buffer size used by discard operations.
const DISCARD_BUFFER_SIZE: usize = 8 * 1024;

/// Reads an indexed range to EOF through a type-erased reader.
///
/// # Parameters
/// - `reader`: Reader that supplies the bytes.
/// - `buffer`: Destination buffer.
/// - `start_index`: Start offset inside `buffer`.
/// - `count`: Number of bytes to try to read.
///
/// # Returns
/// The number of bytes written into the requested range.
///
/// # Errors
/// Returns the first non-interrupted read error.
///
/// # Panics
/// Panics in debug builds if the requested output range does not fit.
///
/// # Safety
/// The caller must guarantee that `start_index..start_index + count` is valid
/// inside `buffer` and that the addition does not overflow.
pub(in crate::std_io::ext) unsafe fn read_exact_or_eof_unchecked_impl(
    reader: &mut dyn Read,
    buffer: &mut [u8],
    start_index: usize,
    count: usize,
) -> Result<usize> {
    debug_assert!(
        SliceRange::range_fits(buffer.len(), start_index, count),
        "unchecked read range exceeds buffer"
    );
    let mut total = 0;
    while total < count {
        // SAFETY: The caller guarantees that `start_index..start_index + count`
        // is valid for `buffer`; `total < count`, so this suffix is valid.
        let target = unsafe { UncheckedSlice::subslice_mut(buffer, start_index + total, count - total) };
        let requested = count - total;
        match reader.read(target) {
            Ok(0) => break,
            Ok(read) => {
                validate_read_count(read, requested)?;
                total += read;
            }
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
///
/// # Type Parameters
/// - `N`: Number of bytes in the returned array.
///
/// # Parameters
/// - `reader`: Reader that supplies the bytes.
///
/// # Returns
/// An array containing exactly `N` bytes.
///
/// # Errors
/// Returns the error reported by [`Read::read_exact`].
#[inline]
pub(in crate::std_io::ext) fn read_exact_array_impl<const N: usize>(reader: &mut dyn Read) -> Result<[u8; N]> {
    let mut buffer = [0; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

/// Reads an exactly sized bounded vector through a type-erased reader.
///
/// # Parameters
/// - `reader`: Reader that supplies the bytes.
/// - `len`: Exact number of bytes to read.
/// - `max_len`: Maximum accepted exact read length.
///
/// # Returns
/// A vector containing exactly `len` bytes.
///
/// # Errors
/// Returns [`ErrorKind::OutOfMemory`] if the result vector cannot be allocated,
/// a read error, or [`ErrorKind::InvalidData`] when `len > max_len`.
#[inline]
pub(in crate::std_io::ext) fn read_exact_vec_limited_impl(
    reader: &mut dyn Read,
    len: usize,
    max_len: usize,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    read_exact_vec_limited_into(reader, &mut output, len, max_len)?;
    Ok(output)
}

/// Discards a bounded byte count through a type-erased reader.
///
/// # Parameters
/// - `reader`: Reader whose bytes are discarded.
/// - `bytes`: Maximum number of bytes to discard.
///
/// # Returns
/// The number of bytes discarded before the limit or EOF was reached.
///
/// # Errors
/// Returns the first non-interrupted read error, or [`ErrorKind::InvalidData`]
/// if the reader reports more bytes than requested.
pub(in crate::std_io::ext) fn discard_exact_or_eof_impl(reader: &mut dyn Read, bytes: u64) -> Result<u64> {
    let mut buffer = [0; DISCARD_BUFFER_SIZE];
    let mut remaining = bytes;
    let mut discarded = 0;
    while remaining > 0 {
        let requested = remaining.min(DISCARD_BUFFER_SIZE as u64) as usize;
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => break,
            Ok(count) => {
                validate_read_count(count, requested)?;
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
///
/// # Parameters
/// - `reader`: Reader that supplies the text bytes.
/// - `max_len`: Maximum accepted byte length.
///
/// # Returns
/// The decoded UTF-8 string.
///
/// # Errors
/// Returns [`ErrorKind::OutOfMemory`] if the result buffer cannot grow, a read
/// error, or [`ErrorKind::InvalidData`] when the input exceeds `max_len` or is
/// not valid UTF-8.
#[inline]
pub(in crate::std_io::ext) fn read_to_string_limited_impl(reader: &mut dyn Read, max_len: usize) -> Result<String> {
    let bytes = read_to_end_limited(reader, max_len)?;
    String::from_utf8(bytes).map_err(invalid_utf8_error)
}

/// Appends bounded UTF-8 text read through a type-erased reader.
///
/// # Parameters
/// - `reader`: Reader that supplies the text bytes.
/// - `output`: String that receives the decoded text.
/// - `max_len`: Maximum accepted byte length.
///
/// # Returns
/// The number of bytes appended to `output`.
///
/// # Errors
/// Returns [`ErrorKind::OutOfMemory`] if the temporary byte buffer or `output`
/// cannot grow, a read error, or [`ErrorKind::InvalidData`] when the input
/// exceeds `max_len` or is not valid UTF-8.
#[inline]
pub(in crate::std_io::ext) fn read_to_string_limited_into_impl(
    reader: &mut dyn Read,
    output: &mut String,
    max_len: usize,
) -> Result<usize> {
    let bytes = read_to_end_limited(reader, max_len)?;
    let text = String::from_utf8(bytes).map_err(invalid_utf8_error)?;
    let count = text.len();
    try_reserve_string(output, count).map_err(allocation_error)?;
    output.push_str(&text);
    Ok(count)
}
