// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, Write};

use crate::util::UncheckedSlice;

/// Extension methods for [`Write`] values.
///
/// `WriteExt` provides small method-style helpers for byte writers. The
/// methods are implemented for every type that implements [`Write`], including
/// `dyn Write` trait objects.
pub trait WriteExt: Write {
    /// Writes bytes from a range of `buffer` without checking the range bounds
    /// in release builds.
    ///
    /// This method delegates to [`Write::write`] after creating the source
    /// slice with raw pointer arithmetic. It performs at most one write
    /// operation and returns the number of bytes written, keeping the same
    /// short-write and error behavior as [`Write::write`].
    ///
    /// # Parameters
    /// - `buffer`: Source buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Maximum number of bytes to write.
    ///
    /// # Returns
    /// The number of bytes written from `buffer[start_index..start_index +
    /// count]`. The value is in `0..=count`.
    ///
    /// # Errors
    /// Returns the error reported by [`Write::write`].
    ///
    /// # Panics
    /// Panics in debug builds if the requested source range does not fit.
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(buffer.len(), start_index, count),
            "unchecked write range exceeds buffer"
        );
        // SAFETY: The caller guarantees that the computed pointer and length
        // form a valid subslice of `buffer`.
        let source = unsafe { UncheckedSlice::subslice(buffer, start_index, count) };
        self.write(source)
    }

    /// Writes exactly `count` bytes from a range of `buffer` without checking
    /// the range bounds in release builds.
    ///
    /// This method delegates to [`Write::write_all`] after creating the source
    /// slice with raw pointer arithmetic. It keeps the same short-write,
    /// [`std::io::ErrorKind::Interrupted`], and
    /// [`std::io::ErrorKind::WriteZero`] behavior as [`Write::write_all`].
    ///
    /// # Parameters
    /// - `buffer`: Source buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Number of bytes to write.
    ///
    /// # Returns
    /// `Ok(())` after the requested range has been written.
    ///
    /// # Errors
    /// Returns the error reported by [`Write::write_all`].
    ///
    /// # Panics
    /// Panics in debug builds if the requested source range does not fit.
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    #[inline(always)]
    unsafe fn write_all_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            UncheckedSlice::range_fits(buffer.len(), start_index, count),
            "unchecked write range exceeds buffer"
        );
        // SAFETY: The caller guarantees that the computed pointer and length
        // form a valid subslice of `buffer`.
        let source = unsafe { UncheckedSlice::subslice(buffer, start_index, count) };
        self.write_all(source)
    }
}

impl<T> WriteExt for T where T: Write + ?Sized {}
