// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    Result,
    Seek,
    SeekFrom,
    Write,
};

use crate::std_io::WriteSeek;

/// Extension methods for values that implement both [`Write`] and [`Seek`].
///
/// `WriteSeekExt` provides position-preserving write helpers for random-access
/// output use cases such as patching headers, offsets, and indexes after a
/// stream has already been written.
pub trait WriteSeekExt: Write + Seek {
    /// Writes all bytes at `offset` and restores the original position.
    ///
    /// This method seeks to `offset`, delegates to [`Write::write_all`], and
    /// then restores the position that was current before the call.
    ///
    /// # Parameters
    /// - `offset`: Absolute byte offset from the start of the stream.
    /// - `buffer`: Bytes to write.
    ///
    /// # Returns
    /// `Ok(())` after all bytes have been written and the position restored.
    ///
    /// # Errors
    /// Returns an error when reading the current position, seeking to `offset`,
    /// writing bytes, or restoring the original position fails. If restoration
    /// fails, the restoration error is returned.
    fn write_all_at_preserving_position(
        &mut self,
        offset: u64,
        buffer: &[u8],
    ) -> Result<()>;
}

/// Implements a position-preserving write through a type-erased stream.
///
/// # Parameters
/// - `output`: Stream to seek, write, and restore.
/// - `offset`: Absolute byte offset to write at.
/// - `buffer`: Bytes to write.
///
/// # Returns
/// `Ok(())` after all bytes have been written and the position restored.
///
/// # Errors
/// Returns an error when querying, changing, or restoring the stream position,
/// or when writing fails.
fn write_all_at_preserving_position_impl(
    output: &mut dyn WriteSeek,
    offset: u64,
    buffer: &[u8],
) -> Result<()> {
    let position = output.stream_position()?;
    let write_result = match output.seek(SeekFrom::Start(offset)) {
        Ok(_) => output.write_all(buffer),
        Err(error) => Err(error),
    };
    let restore_result = output.seek(SeekFrom::Start(position));
    match (write_result, restore_result) {
        (Ok(()), Ok(_)) => Ok(()),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

impl<T> WriteSeekExt for T
where
    T: Write + Seek + ?Sized,
{
    /// Writes at an absolute offset and restores the original position.
    ///
    /// # Parameters
    /// - `offset`: Absolute byte offset to write at.
    /// - `buffer`: Bytes to write.
    ///
    /// # Returns
    /// `Ok(())` after all bytes have been written and the position restored.
    ///
    /// # Errors
    /// Returns an error when querying, changing, or restoring the stream
    /// position, or when writing fails.
    #[inline(always)]
    fn write_all_at_preserving_position(
        &mut self,
        offset: u64,
        buffer: &[u8],
    ) -> Result<()> {
        let mut output = self;
        write_all_at_preserving_position_impl(&mut output, offset, buffer)
    }
}
