// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Extension operations for inspecting standard seekable streams.

use std::io::Result;
use std::io::Seek;

use crate::std_io::ext::internal::seek_ext_impl;

/// Extension methods for [`Seek`] values.
///
/// `SeekExt` provides stable, position-preserving helpers for seekable streams.
/// The methods are implemented for every type that implements [`Seek`],
/// including `dyn Seek` trait objects.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use std::io::Seek;
///
/// use qubit_io::std_io::ext::SeekExt;
///
/// let mut stream = Cursor::new(b"hello".to_vec());
/// stream.set_position(2);
/// assert_eq!(stream.stream_size()?, 5);
/// assert_eq!(stream.stream_position()?, 2);
/// # Ok::<(), std::io::Error>(())
/// ```
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
    T: Seek + ?Sized,
{
    /// Gets the stream size without changing its final position.
    ///
    /// # Returns
    /// The stream size in bytes.
    ///
    /// # Errors
    /// Returns an error when querying, changing, or restoring the stream
    /// position fails.
    #[inline(always)]
    fn stream_size(&mut self) -> Result<u64> {
        let mut stream = self;
        seek_ext_impl::stream_size(&mut stream)
    }
}
