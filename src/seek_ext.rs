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
    /// Gets the stream length without changing the final stream position.
    ///
    /// The original position is captured with [`Seek::stream_position`], then
    /// the stream is moved to the end to measure its length, and finally the
    /// original position is restored.
    ///
    /// # Returns
    /// The stream length in bytes.
    ///
    /// # Errors
    /// Returns an error when reading the current position, seeking to the end,
    /// or restoring the original position fails. If restoring fails, the
    /// restore error is returned because the caller's stream position contract
    /// was not preserved.
    fn stream_len_preserving_position(&mut self) -> Result<u64>;
}

impl<T> SeekExt for T
where
    T: Seek,
{
    #[inline]
    fn stream_len_preserving_position(&mut self) -> Result<u64> {
        stream_len_preserving_position_impl(self)
    }
}

impl SeekExt for dyn Seek + '_ {
    #[inline]
    fn stream_len_preserving_position(&mut self) -> Result<u64> {
        stream_len_preserving_position_impl(self)
    }
}

/// Gets the length of `stream` and restores its original position.
///
/// # Parameters
/// - `stream`: Seekable stream to inspect.
///
/// # Returns
/// Stream length in bytes.
///
/// # Errors
/// Returns an error when position lookup, end seeking, or restoration fails.
fn stream_len_preserving_position_impl(stream: &mut dyn Seek) -> Result<u64> {
    let position = stream.stream_position()?;
    let length_result = stream.seek(SeekFrom::End(0));
    let restore_result = stream.seek(SeekFrom::Start(position));
    match (length_result, restore_result) {
        (Ok(length), Ok(_)) => Ok(length),
        (Err(error), Ok(_)) => Err(error),
        (_, Err(error)) => Err(error),
    }
}
