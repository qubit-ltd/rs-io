/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::cmp::Ordering;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
    copy,
};
use std::string::FromUtf8Error;

use super::allocation::try_reserve_vec;
use crate::{
    Leb128DecodeError,
    ReadExt,
};

/// Default buffer size used by stream copy operations.
const COPY_BUFFER_SIZE: usize = 16 * 1024;

/// Buffer size used by stream comparison operations.
const COMPARE_BUFFER_SIZE: usize = 16 * 1024;

/// Stream utility namespace.
///
/// This type is an uninstantiable namespace for operations involving one or
/// more [`Read`] or [`Write`] values. The methods do not close or flush the
/// supplied streams unless the underlying standard-library operation documents
/// otherwise.
///
/// # Examples
/// ```
/// use qubit_io::Streams;
/// use std::io::Cursor;
///
/// let mut input = Cursor::new(b"abcdef".to_vec());
/// let mut output = Vec::new();
///
/// let copied = Streams::copy_at_most(&mut input, &mut output, 4)?;
///
/// assert_eq!(4, copied);
/// assert_eq!(b"abcd", output.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub enum Streams {}

impl Streams {
    /// Copies all remaining bytes from `reader` to `writer`.
    ///
    /// This is a namespace-style wrapper around [`std::io::copy`]. It preserves
    /// the standard-library behavior, including platform-specific optimized
    /// copy paths when available.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns the first read or write error reported by the underlying
    /// streams, using the same error behavior as [`std::io::copy`].
    #[inline]
    pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        copy(reader, writer)
    }

    /// Copies at most `max_bytes` bytes from `reader` to `writer`.
    ///
    /// This method stops successfully when either EOF is reached or
    /// `max_bytes` bytes have been copied. It does not close or flush either
    /// stream.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum number of bytes to copy.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns the first non-interrupted read error or write error reported by
    /// the underlying streams. Interrupted reads are retried.
    #[inline]
    pub fn copy_at_most<R, W>(reader: &mut R, writer: &mut W, max_bytes: u64) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_at_most_impl(&mut reader, &mut writer, max_bytes)
    }

    /// Copies the remaining input if its total length is at most `max_bytes`.
    ///
    /// This method copies from the current reader position until EOF. If EOF is
    /// not reached within `max_bytes` bytes, it returns
    /// [`std::io::ErrorKind::InvalidData`]. Detecting oversized input consumes
    /// one excess byte from `reader`; that excess byte is not written to
    /// `writer`.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum accepted number of bytes in the remaining input.
    ///
    /// # Returns
    /// The number of bytes copied when EOF is reached within the limit.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidData`] when the remaining input is
    /// longer than `max_bytes`. Returns the first non-interrupted read error or
    /// write error reported by the underlying streams. Interrupted reads are
    /// retried.
    #[inline]
    pub fn copy_to_end_limited<R, W>(reader: &mut R, writer: &mut W, max_bytes: u64) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_to_end_limited_impl(&mut reader, &mut writer, max_bytes)
    }

    /// Tests whether two readable streams have equal remaining contents.
    ///
    /// The comparison starts at each reader's current position and consumes
    /// both streams until a difference or EOF is found.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    ///
    /// # Returns
    /// `true` when both streams produce the same bytes until EOF.
    ///
    /// # Errors
    /// Returns the first read error reported by either stream.
    #[inline]
    pub fn content_eq(left: &mut dyn Read, right: &mut dyn Read) -> Result<bool> {
        Ok(Self::compare_content(left, right)? == Ordering::Equal)
    }

    /// Lexicographically compares the remaining contents of two readable
    /// streams.
    ///
    /// The comparison starts at each reader's current position and consumes
    /// both streams until a difference or EOF is found.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    ///
    /// # Returns
    /// The lexicographic ordering of the remaining bytes.
    ///
    /// # Errors
    /// Returns the first read error reported by either stream.
    pub fn compare_content(left: &mut dyn Read, right: &mut dyn Read) -> Result<Ordering> {
        let mut left_buffer = [0; COMPARE_BUFFER_SIZE];
        let mut right_buffer = [0; COMPARE_BUFFER_SIZE];
        loop {
            let left_count = left.read_exact_or_eof(&mut left_buffer)?;
            let right_count = right.read_exact_or_eof(&mut right_buffer)?;
            let n = left_count.min(right_count);
            for index in 0..n {
                match left_buffer[index].cmp(&right_buffer[index]) {
                    Ordering::Equal => {}
                    ordering => return Ok(ordering),
                }
            }
            match left_count.cmp(&right_count) {
                Ordering::Equal if left_count == 0 => return Ok(Ordering::Equal),
                Ordering::Equal => {}
                ordering => return Ok(ordering),
            }
        }
    }
}

/// Copies at most `max_bytes` bytes using trait-object I/O endpoints.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `writer`: Destination writer.
/// - `max_bytes`: Maximum number of bytes to copy.
///
/// # Returns
/// The number of bytes copied.
///
/// # Errors
/// Returns the first non-interrupted read error or write error reported by the
/// underlying streams. Interrupted reads are retried.
fn copy_at_most_impl(reader: &mut dyn Read, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
    let mut buffer = [0; COPY_BUFFER_SIZE];
    let mut remaining = max_bytes;
    let mut copied = 0;
    while remaining > 0 {
        let requested = remaining.min(COPY_BUFFER_SIZE as u64) as usize;
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => break,
            Ok(count) => {
                writer.write_all(&buffer[..count])?;
                let count = count as u64;
                remaining -= count;
                copied += count;
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Ok(copied)
}

/// Copies the remaining input through trait-object endpoints when it fits.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `writer`: Destination writer.
/// - `max_bytes`: Maximum accepted number of bytes in the remaining input.
///
/// # Returns
/// The number of bytes copied when EOF is reached within the limit.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] when the remaining input is longer than
/// `max_bytes`. Returns the first non-interrupted read error or write error
/// reported by the underlying streams. Interrupted reads are retried.
fn copy_to_end_limited_impl(reader: &mut dyn Read, writer: &mut dyn Write, max_bytes: u64) -> Result<u64> {
    let copied = copy_at_most_impl(reader, writer, max_bytes)?;
    if copied < max_bytes {
        return Ok(copied);
    }
    if has_more_input(reader)? {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("input exceeds maximum length of {max_bytes} bytes"),
        ));
    }
    Ok(copied)
}

/// Returns whether `reader` has at least one more byte.
///
/// # Parameters
/// - `reader`: Source reader to probe.
///
/// # Returns
/// `true` when one extra byte was read, or `false` when EOF was reached.
///
/// # Errors
/// Returns the first non-interrupted read error reported by `reader`.
fn has_more_input(reader: &mut dyn Read) -> Result<bool> {
    let mut byte = [0];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return Ok(false),
            Ok(_) => return Ok(true),
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
}

/// Reads one terminated LEB128 payload from a byte stream.
///
/// The function fills a fixed-size stack buffer one byte at a time until a
/// terminating byte is found or the buffer is full, then delegates decoding to
/// `decode`.
///
/// # Parameters
///
/// - `reader`: Source reader.
/// - `decode`: Decoder for the populated stack buffer.
///
/// # Returns
///
/// Returns the decoded value.
///
/// # Errors
///
/// Returns an I/O error reported by `reader`, or [`ErrorKind::InvalidData`] when
/// `decode` rejects the payload.
#[inline]
pub(crate) fn read_leb128_payload<const N: usize, T, R, F>(reader: &mut R, decode: F) -> Result<T>
where
    R: Read + ?Sized,
    F: FnOnce(&[u8]) -> std::result::Result<(T, usize), Leb128DecodeError>,
{
    let mut bytes = [0u8; N];
    for index in 0..N {
        let target = one_byte_slice(&mut bytes, index);
        reader.read_exact(target)?;
        if bytes[index] & 0x80 == 0 {
            return decode(&bytes)
                .map(|(value, _)| value)
                .map_err(|error| Error::new(ErrorKind::InvalidData, error));
        }
    }
    decode(&bytes)
        .map(|(value, _)| value)
        .map_err(|error| Error::new(ErrorKind::InvalidData, error))
}

/// Creates a mutable one-byte slice at `index`.
///
/// # Parameters
///
/// - `bytes`: Fixed-size temporary buffer.
/// - `index`: Byte index inside `bytes`.
///
/// # Returns
///
/// Returns a mutable slice containing exactly `bytes[index]`.
#[inline]
fn one_byte_slice(bytes: &mut [u8], index: usize) -> &mut [u8] {
    // SAFETY: Callers pass an index inside the fixed-size local buffer.
    unsafe { core::slice::from_raw_parts_mut(bytes.as_mut_ptr().add(index), 1) }
}

/// Reads a UTF-8 payload after its length has already been decoded.
///
/// # Parameters
///
/// - `reader`: Reader that provides the UTF-8 payload bytes.
/// - `len`: Payload length in bytes.
/// - `max_len`: Maximum accepted payload length in bytes.
///
/// # Returns
///
/// Returns the decoded UTF-8 string.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when `len` exceeds `max_len`, an
/// allocation error when reserving the output buffer fails, an I/O error from
/// `reader`, or [`ErrorKind::InvalidData`] when the payload is not valid UTF-8.
pub(crate) fn read_utf8_payload<R>(reader: &mut R, len: usize, max_len: usize) -> Result<String>
where
    R: Read + ?Sized,
{
    if len > max_len {
        return Err(length_exceeded_error(len, max_len));
    }
    let mut bytes = Vec::new();
    try_reserve_vec(&mut bytes, len)?;
    bytes.resize(len, 0);
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(invalid_utf8_error)
}

/// Writes a UTF-8 payload without a length prefix.
///
/// # Parameters
///
/// - `writer`: Destination writer.
/// - `value`: String slice to write.
///
/// # Errors
///
/// Returns the I/O error reported by `writer`.
pub(crate) fn write_utf8_payload<W>(writer: &mut W, value: &str) -> Result<()>
where
    W: Write + ?Sized,
{
    writer.write_all(value.as_bytes())
}

/// Writes a UTF-8 string after a `u16` byte-length prefix.
///
/// # Parameters
///
/// - `writer`: Destination writer.
/// - `value`: String slice to write.
/// - `write_len`: Callback that writes the encoded `u16` length.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not fit
/// into `u16`, or an I/O error from the underlying writer.
pub(crate) fn write_utf8_string_with_u16_len<W, F>(writer: &mut W, value: &str, write_len: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut W, u16) -> Result<()>,
{
    let bytes = value.as_bytes();
    write_len(writer, checked_u16_len(bytes.len())?)?;
    writer.write_all(bytes)
}

/// Writes a UTF-8 string after a `u32` byte-length prefix.
///
/// # Parameters
///
/// - `writer`: Destination writer.
/// - `value`: String slice to write.
/// - `write_len`: Callback that writes the encoded `u32` length.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when the UTF-8 byte length does not fit
/// into `u32`, or an I/O error from the underlying writer.
pub(crate) fn write_utf8_string_with_u32_len<W, F>(writer: &mut W, value: &str, write_len: F) -> Result<()>
where
    W: Write + ?Sized,
    F: FnOnce(&mut W, u32) -> Result<()>,
{
    let bytes = value.as_bytes();
    write_len(writer, checked_u32_len(bytes.len())?)?;
    writer.write_all(bytes)
}

/// Converts a UTF-8 payload length to a `u16` length prefix value.
///
/// # Parameters
///
/// - `len`: Payload length in bytes.
///
/// # Returns
///
/// Returns the payload length represented as `u16`.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when `len` is larger than `u16::MAX`.
pub(crate) fn checked_u16_len(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u16 length"),
        )
    })
}

/// Converts a UTF-8 payload length to a `u32` length prefix value.
///
/// # Parameters
///
/// - `len`: Payload length in bytes.
///
/// # Returns
///
/// Returns the payload length represented as `u32`.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when `len` is larger than `u32::MAX`.
pub(crate) fn checked_u32_len(len: usize) -> Result<u32> {
    if len > u32::MAX as usize {
        Err(Error::new(
            ErrorKind::InvalidInput,
            format!("string length {len} exceeds maximum encodable u32 length"),
        ))
    } else {
        Ok(len as u32)
    }
}

/// Builds an invalid-data error for UTF-8 payloads that exceed their limit.
///
/// # Parameters
///
/// - `len`: Decoded payload length.
/// - `max_len`: Maximum accepted payload length.
///
/// # Returns
///
/// Returns an [`ErrorKind::InvalidData`] error.
fn length_exceeded_error(len: usize, max_len: usize) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("string length {len} exceeds maximum length of {max_len} bytes"),
    )
}

/// Converts an invalid UTF-8 payload error into an I/O error.
///
/// # Parameters
///
/// - `error`: UTF-8 conversion error.
///
/// # Returns
///
/// Returns an [`ErrorKind::InvalidData`] error containing the UTF-8 error
/// context.
fn invalid_utf8_error(error: FromUtf8Error) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("length-prefixed string is not valid UTF-8: {error}"),
    )
}
