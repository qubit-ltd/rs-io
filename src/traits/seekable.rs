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
};

/// Minimal seek interface measured in stream units.
///
/// Unlike [`Seek`], which measures positions and offsets in bytes,
/// `Seekable` measures them in units of [`Self::Item`]. For byte streams,
/// set `Item = u8`; offsets passed through [`SeekFrom`] then count units
/// rather than bytes.
///
/// The return value of [`Seekable::seek`] is the new absolute position from
/// the start of the stream, in units.
pub trait Seekable {
    /// The unit type used to measure seek positions and offsets.
    type Item;

    /// Seeks to a position in the stream.
    ///
    /// # Parameters
    ///
    /// * `position` - Target position relative to the start, end, or current
    ///   logical position. All offsets are counted in units of [`Self::Item`].
    ///
    /// # Returns
    ///
    /// The new absolute stream position, in units.
    ///
    /// # Errors
    ///
    /// Returns the seek error reported by the implementation.
    fn seek(&mut self, position: SeekFrom) -> Result<u64>;
}

impl<S> Seekable for S
where
    S: Seek + ?Sized,
{
    type Item = u8;

    /// Seeks a standard [`Seek`] value using byte offsets.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        Seek::seek(self, position)
    }
}
