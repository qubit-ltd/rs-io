// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal implementation for [`crate::std_io::ext::SeekExt`].

use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

/// Measures a stream through a type-erased seekable stream.
///
/// # Parameters
/// - `stream`: Stream to measure and restore.
///
/// # Returns
/// The stream size in bytes.
///
/// # Errors
/// Returns an error when querying the current position, seeking to the end, or
/// restoring the original position fails. If restoring fails, the restore
/// error takes precedence because the stream's original position was not
/// preserved.
pub(crate) fn stream_size(stream: &mut dyn Seek) -> Result<u64> {
    let position = stream.stream_position()?;
    let size_result = stream.seek(SeekFrom::End(0));
    let restore_result = stream.seek(SeekFrom::Start(position));
    match (size_result, restore_result) {
        (Ok(size), Ok(_)) => Ok(size),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}
