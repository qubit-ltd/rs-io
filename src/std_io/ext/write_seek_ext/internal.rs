// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private implementation for position-preserving writer operations.

use std::io::Result;
use std::io::SeekFrom;

use crate::std_io::WriteSeek;

/// Writes at an absolute offset through a type-erased stream and restores its
/// position.
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
pub(super) fn write_all_at_preserving_position_impl(
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
