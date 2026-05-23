/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{ErrorKind, Read, Result, Seek, SeekFrom};

use crate::ReadSeek;

/// Extension methods for values that implement both [`Read`] and [`Seek`].
///
/// `ReadSeekExt` provides position-preserving read helpers for common
/// inspection use cases such as file signature checks, MIME detection, and
/// random-offset probing.
pub trait ReadSeekExt: Read + Seek {
    /// Reads from the current position and restores the original position.
    ///
    /// This method has the same partial-EOF semantics as
    /// [`crate::ReadExt::read_exact_or_eof`], but it leaves the stream positioned
    /// where it was before the call when restoration succeeds.
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

impl<T> ReadSeekExt for T
where
    T: Read + Seek + ?Sized,
{
    #[inline]
    fn peek_exact_or_eof(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        peek_exact_or_eof_impl(&mut reader, buffer)
    }

    #[inline]
    fn read_exact_or_eof_at(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let mut reader = self;
        read_exact_or_eof_at_impl(&mut reader, offset, buffer)
    }
}

/// Reads from the current stream position and restores that position.
///
/// # Parameters
/// - `reader`: Seekable reader to inspect.
/// - `buffer`: Destination buffer to fill.
///
/// # Returns
/// The number of bytes written into `buffer`.
///
/// # Errors
/// Returns an error when position lookup, reading, or position restoration fails.
fn peek_exact_or_eof_impl(reader: &mut dyn ReadSeek, buffer: &mut [u8]) -> Result<usize> {
    let position = reader.stream_position()?;
    let read_result = read_exact_or_eof(reader, buffer);
    let restore_result = reader.seek(SeekFrom::Start(position));
    match (read_result, restore_result) {
        (Ok(count), Ok(_)) => Ok(count),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

/// Reads from `offset` and restores the original stream position.
///
/// # Parameters
/// - `reader`: Seekable reader to inspect.
/// - `offset`: Absolute byte offset from the start of the stream.
/// - `buffer`: Destination buffer to fill.
///
/// # Returns
/// The number of bytes written into `buffer`.
///
/// # Errors
/// Returns an error when position lookup, seeking, reading, or position restoration fails.
fn read_exact_or_eof_at_impl(
    reader: &mut dyn ReadSeek,
    offset: u64,
    buffer: &mut [u8],
) -> Result<usize> {
    let position = reader.stream_position()?;
    let read_result = match reader.seek(SeekFrom::Start(offset)) {
        Ok(_) => read_exact_or_eof(reader, buffer),
        Err(error) => Err(error),
    };
    let restore_result = reader.seek(SeekFrom::Start(position));
    match (read_result, restore_result) {
        (Ok(count), Ok(_)) => Ok(count),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
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
fn read_exact_or_eof(reader: &mut dyn ReadSeek, buffer: &mut [u8]) -> Result<usize> {
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
