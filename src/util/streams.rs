// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::cmp::Ordering;
use std::io::{Error, ErrorKind, Read, Result, Write};

use crate::ReadExt;
use crate::capacity_const::{DEFAULT_COMPARE_BUFFER_SIZE, DEFAULT_COPY_BUFFER_SIZE};
use crate::util::create_vec;

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
        std::io::copy(reader, writer)
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
        copy_at_most_impl(
            &mut reader,
            &mut writer,
            max_bytes,
            DEFAULT_COPY_BUFFER_SIZE,
        )
    }

    /// Copies at most `max_bytes` bytes from `reader` to `writer` using a
    /// caller-selected heap buffer.
    ///
    /// This method has the same copy semantics as [`Self::copy_at_most`], but
    /// allocates a buffer on the heap with `buffer_size` bytes. Use it when
    /// the default chunk size is too large for the caller's stack budget or
    /// when a smaller copy window is desirable.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum number of bytes to copy.
    /// - `buffer_size`: Number of bytes in the copy buffer.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
    /// allocation error if the copy buffer cannot be allocated. Returns the
    /// first non-interrupted read error or write error reported by the
    /// underlying streams. Interrupted reads are retried.
    #[inline]
    pub fn copy_at_most_with_buffer_size<R, W>(
        reader: &mut R,
        writer: &mut W,
        max_bytes: u64,
        buffer_size: usize,
    ) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_at_most_impl(&mut reader, &mut writer, max_bytes, buffer_size)
    }

    /// Copies the remaining input if its total length is at most `max_bytes`.
    ///
    /// This method copies from the current reader position until EOF. If EOF is
    /// not reached within `max_bytes` bytes, it returns
    /// [`std::io::ErrorKind::InvalidData`]. Detecting oversized input consumes
    /// one excess byte from `reader`; that excess byte is not written to
    /// `writer`.
    ///
    /// Unlike bounded reads into in-memory collections, this method cannot roll
    /// back bytes already accepted by `writer` when the limit is exceeded
    /// because [`Write`] does not provide truncation. On
    /// [`std::io::ErrorKind::InvalidData`], up to `max_bytes` bytes may remain
    /// in `writer`.
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
        let copied = copy_at_most_impl(
            &mut reader,
            &mut writer,
            max_bytes,
            DEFAULT_COPY_BUFFER_SIZE,
        )?;
        if copied < max_bytes {
            return Ok(copied);
        }
        let mut byte = [0];
        loop {
            match reader.read(&mut byte) {
                Ok(0) => return Ok(copied),
                Ok(_) => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("input exceeds maximum length of {max_bytes} bytes"),
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

    /// Tests whether two readable streams have equal remaining contents.
    ///
    /// The comparison starts at each reader's current position and reads both
    /// streams in fixed-size chunks. A mismatch stops comparison immediately
    /// after the differing chunks are read, so each reader may have advanced
    /// past the first differing byte within that chunk.
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
    /// The comparison starts at each reader's current position and reads both
    /// streams in fixed-size chunks. A mismatch stops comparison immediately
    /// after the differing chunks are read, so each reader may have advanced
    /// past the first differing byte within that chunk.
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
        Self::compare_content_with_buffer_size(left, right, DEFAULT_COMPARE_BUFFER_SIZE)
    }

    /// Lexicographically compares the remaining contents of two readable
    /// streams using caller-selected heap buffers.
    ///
    /// This method has the same comparison and stream-advance semantics as
    /// [`Self::compare_content`], but allocates two buffers on the heap with
    /// `buffer_size` bytes each. Use it when the default chunk size is too
    /// large for the caller's stack budget or when a smaller comparison window
    /// is desirable.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    /// - `buffer_size`: Number of bytes in each comparison buffer.
    ///
    /// # Returns
    /// The lexicographic ordering of the remaining bytes.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
    /// allocation error if the comparison buffers cannot be allocated. Returns
    /// the first read error reported by either stream.
    pub fn compare_content_with_buffer_size(
        left: &mut dyn Read,
        right: &mut dyn Read,
        buffer_size: usize,
    ) -> Result<Ordering> {
        if buffer_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "compare buffer size must be greater than zero",
            ));
        }
        let mut left_buffer = create_vec(buffer_size, 0)?;
        let mut right_buffer = create_vec(buffer_size, 0)?;
        debug_assert_eq!(
            left_buffer.len(),
            right_buffer.len(),
            "compare buffers must have identical lengths",
        );
        debug_assert!(!left_buffer.is_empty(), "compare buffers must not be empty",);
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
                Ordering::Equal if left_count == 0 => {
                    return Ok(Ordering::Equal);
                }
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
/// - `buffer_size`: Number of bytes in the copy buffer.
///
/// # Returns
/// The number of bytes copied.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
/// allocation error if the copy buffer cannot be allocated. Returns the first
/// non-interrupted read error or write error reported by the underlying
/// streams. Interrupted reads are retried.
fn copy_at_most_impl(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    max_bytes: u64,
    buffer_size: usize,
) -> Result<u64> {
    if buffer_size == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "copy buffer size must be greater than zero",
        ));
    }
    let mut buffer = create_vec(buffer_size, 0)?;
    let mut remaining = max_bytes;
    let mut copied = 0;
    while remaining > 0 {
        let requested = remaining.min(buffer_size as u64) as usize;
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
