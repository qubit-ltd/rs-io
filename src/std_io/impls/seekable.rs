// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Standard [`Seek`](std::io::Seek) implementations of item seeking traits.

use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

use crate::Seekable;

/// Bridges byte-oriented standard seeking to item-oriented seeking.
impl<S> Seekable for S
where
    S: Seek + ?Sized,
{
    /// Bytes used by the standard Seek implementation.
    type Unit = u8;

    /// Seeks using the standard byte offset semantics.
    ///
    /// # Parameters
    ///
    /// - `position`: Standard-library seek origin and byte offset.
    ///
    /// # Returns
    ///
    /// The resulting byte offset from the start of the stream.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the standard seeker.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        Seek::seek(self, position)
    }
}
