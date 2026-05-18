/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    Result,
    Seek,
    SeekFrom,
};

/// Extension methods for [`Seek`] values.
///
/// `SeekExt` provides stable, position-preserving helpers for seekable streams.
/// The methods are implemented for every type that implements [`Seek`],
/// including `dyn Seek` trait objects.
pub trait SeekExt: Seek {
    /// Gets the stream size without changing the final stream position.
    ///
    /// The original position is captured with [`Seek::stream_position`], then
    /// the stream is moved to the end to measure its size, and finally the
    /// original position is restored.
    ///
    /// # Compatibility
    /// If the standard library adds an equivalent `Seek::stream_size` method,
    /// this extension method may be deprecated in a future release.
    ///
    /// # Returns
    /// The stream size in bytes.
    ///
    /// # Errors
    /// Returns an error when reading the current position, seeking to the end,
    /// or restoring the original position fails. If restoring fails, the
    /// restore error is returned because the caller's stream position contract
    /// was not preserved.
    fn stream_size(&mut self) -> Result<u64>;
}

impl<T> SeekExt for T
where
    T: Seek,
{
    #[inline]
    fn stream_size(&mut self) -> Result<u64> {
        stream_size_impl(self)
    }
}

/// Gets the size of `stream` and restores its original position.
///
/// # Parameters
/// - `stream`: Seekable stream to inspect.
///
/// # Returns
/// Stream size in bytes.
///
/// # Errors
/// Returns an error when position lookup, end seeking, or restoration fails.
fn stream_size_impl(stream: &mut dyn Seek) -> Result<u64> {
    let position = stream.stream_position()?;
    let size_result = stream.seek(SeekFrom::End(0));
    let restore_result = stream.seek(SeekFrom::Start(position));
    match (size_result, restore_result) {
        (Ok(size), Ok(_)) => Ok(size),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}
