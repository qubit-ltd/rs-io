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
    Read,
    Result,
    Write,
    copy,
};

use crate::util::{
    compare,
    copy as copy_util,
};

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
        copy_util::copy_at_most(reader, writer, max_bytes)
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
        copy_util::copy_to_end_limited(reader, writer, max_bytes)
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
        compare::content_eq(left, right)
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
    #[inline]
    pub fn compare_content(left: &mut dyn Read, right: &mut dyn Read) -> Result<Ordering> {
        compare::compare_content(left, right)
    }
}
