// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Position-preserving extension operations for seekable standard writers.

use std::io::Result;
use std::io::Seek;
use std::io::Write;

mod internal;

/// Extension methods for values that implement both [`Write`] and [`Seek`].
///
/// `WriteSeekExt` provides position-preserving write helpers for random-access
/// output use cases such as patching headers, offsets, and indexes after a
/// stream has already been written.
///
/// # Examples
/// ```
/// use qubit_io::std_io::ext::WriteSeekExt;
/// use std::io::Cursor;
///
/// let mut output = Cursor::new(b"abcdef".to_vec());
/// output.set_position(2);
///
/// output.write_all_at_preserving_position(1, b"XY")?;
///
/// assert_eq!(output.get_ref(), b"aXYdef");
/// assert_eq!(output.position(), 2);
/// # Ok::<(), std::io::Error>(())
/// ```
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
    /// Returns `Ok(())` after all bytes have been written and the original
    /// stream position has been restored.
    ///
    /// # Errors
    /// Returns an error when reading the current position, seeking to `offset`,
    /// writing bytes, or restoring the original position fails. If restoration
    /// fails, the restoration error is returned.
    fn write_all_at_preserving_position(&mut self, offset: u64, buffer: &[u8]) -> Result<()>;
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
    fn write_all_at_preserving_position(&mut self, offset: u64, buffer: &[u8]) -> Result<()> {
        let mut output = self;
        internal::write_all_at_preserving_position_impl(&mut output, offset, buffer)
    }
}
