// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{Read, Result, Seek, SeekFrom};

use crate::std_io::{ReadSeek, ext::internal::read_ext_impl};

/// Extension methods for values that implement both [`Read`] and [`Seek`].
///
/// `ReadSeekExt` provides position-preserving read helpers for common
/// inspection use cases such as file signature checks, MIME detection, and
/// random-offset probing.
pub trait ReadSeekExt: Read + Seek {
    /// Reads from the current position and restores the original position.
    ///
    /// This method has the same partial-EOF semantics as
    /// [`crate::std_io::ext::ReadExt::read_exact_or_eof`], but it leaves the stream
    /// positioned where it was before the call when restoration succeeds.
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer to fill.
    ///
    /// # Returns
    /// The number of bytes written into `buffer`.
    ///
    /// # Errors
    /// Returns an error when reading the current position, reading bytes, or
    /// restoring the original position fails. If both reading and restoration
    /// fail, the restoration error is returned because the caller's stream
    /// position contract was not preserved.
    fn peek_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Reads from `offset` and restores the original position.
    ///
    /// This method seeks to `offset`, reads until `buffer` is full or EOF is
    /// reached, and then restores the position that was current before the
    /// call.
    ///
    /// # Parameters
    /// - `offset`: Absolute byte offset from the start of the stream.
    /// - `buffer`: Destination buffer to fill.
    ///
    /// # Returns
    /// The number of bytes written into `buffer`.
    ///
    /// # Errors
    /// Returns an error when reading the current position, seeking to `offset`,
    /// reading bytes, or restoring the original position fails. If restoration
    /// fails, the restoration error is returned.
    fn read_exact_or_eof_at(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize>;
}

/// Implements a position-preserving read through a type-erased stream.
///
/// # Parameters
/// - `reader`: Stream to read and restore.
/// - `buffer`: Destination buffer.
///
/// # Returns
/// The number of bytes written into `buffer`.
///
/// # Errors
/// Returns an error when reading, querying, or restoring the stream position
/// fails.
fn peek_exact_or_eof_impl(reader: &mut dyn ReadSeek, buffer: &mut [u8]) -> Result<usize> {
    let position = reader.stream_position()?;
    let read_result = read_ext_impl::read_exact_or_eof(reader, buffer);
    let restore_result = reader.seek(SeekFrom::Start(position));
    match (read_result, restore_result) {
        (Ok(count), Ok(_)) => Ok(count),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

/// Implements an offset read through a type-erased stream.
///
/// # Parameters
/// - `reader`: Stream to seek, read, and restore.
/// - `offset`: Absolute byte offset to read from.
/// - `buffer`: Destination buffer.
///
/// # Returns
/// The number of bytes written into `buffer`.
///
/// # Errors
/// Returns an error when querying, changing, or restoring the stream position,
/// or when reading fails.
fn read_exact_or_eof_at_impl(
    reader: &mut dyn ReadSeek,
    offset: u64,
    buffer: &mut [u8],
) -> Result<usize> {
    let position = reader.stream_position()?;
    let read_result = match reader.seek(SeekFrom::Start(offset)) {
        Ok(_) => read_ext_impl::read_exact_or_eof(reader, buffer),
        Err(error) => Err(error),
    };
    let restore_result = reader.seek(SeekFrom::Start(position));
    match (read_result, restore_result) {
        (Ok(count), Ok(_)) => Ok(count),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

impl<T> ReadSeekExt for T
where
    T: Read + Seek + ?Sized,
{
    /// Reads from the current position and restores it afterward.
    ///
    /// # Parameters
    /// - `buffer`: Destination buffer.
    ///
    /// # Returns
    /// The number of bytes written into `buffer`.
    ///
    /// # Errors
    /// Returns an error when reading, querying, or restoring the stream
    /// position fails.
    #[inline(always)]
    fn peek_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        peek_exact_or_eof_impl(&mut reader, buffer)
    }

    /// Reads from an absolute offset and restores the original position.
    ///
    /// # Parameters
    /// - `offset`: Absolute byte offset to read from.
    /// - `buffer`: Destination buffer.
    ///
    /// # Returns
    /// The number of bytes written into `buffer`.
    ///
    /// # Errors
    /// Returns an error when querying, changing, or restoring the stream
    /// position, or when reading fails.
    #[inline(always)]
    fn read_exact_or_eof_at(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        read_exact_or_eof_at_impl(&mut reader, offset, buffer)
    }
}
