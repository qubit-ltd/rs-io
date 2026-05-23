/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{Error, ErrorKind, Result, Write};

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
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    unsafe fn write_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Writes exactly `count` bytes from a range of `buffer` without checking
    /// the range bounds in release builds.
    ///
    /// This method repeatedly calls [`Write::write`] with unchecked source
    /// subslices until the requested range has been written or an error occurs.
    /// It keeps the same short-write, [`ErrorKind::Interrupted`], and
    /// [`ErrorKind::WriteZero`] behavior as [`Write::write_all`].
    ///
    /// # Parameters
    /// - `buffer`: Source buffer.
    /// - `start_index`: Start offset inside `buffer`.
    /// - `count`: Number of bytes to write.
    ///
    /// # Errors
    /// Returns the error reported by [`Write::write_all`].
    ///
    /// # Safety
    /// The caller must guarantee that `start_index..start_index + count` is a
    /// valid range within `buffer` and that `start_index + count` does not
    /// overflow `usize`.
    unsafe fn write_all_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<()>;
}

impl<T> WriteExt for T
where
    T: Write + ?Sized,
{
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: The caller guarantees that the requested range is valid for
        // `buffer`.
        let source = unsafe { slice_unchecked(buffer, start_index, count) };
        self.write(source)
    }

    unsafe fn write_all_unchecked(
        &mut self,
        buffer: &[u8],
        start_index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            start_index
                .checked_add(count)
                .is_some_and(|end_index| end_index <= buffer.len()),
            "unchecked write range exceeds buffer"
        );
        let base = unsafe { buffer.as_ptr().add(start_index) };
        let mut total = 0;
        while total < count {
            // SAFETY: The caller guarantees that `start_index..start_index +
            // count` is valid for `buffer`; `total < count`, so this remaining
            // suffix is also a valid subslice.
            let source = unsafe { core::slice::from_raw_parts(base.add(total), count - total) };
            match self.write(source) {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(written) => total += written,
                Err(error) => {
                    if error.kind() == ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

/// Returns an unchecked immutable range of `buffer`.
///
/// # Parameters
/// - `buffer`: Source byte slice.
/// - `start_index`: Start offset inside `buffer`.
/// - `count`: Number of bytes in the returned range.
///
/// # Safety
/// The caller must guarantee that `start_index..start_index + count` is a
/// valid range within `buffer` and that `start_index + count` does not overflow
/// `usize`.
#[inline(always)]
unsafe fn slice_unchecked(buffer: &[u8], start_index: usize, count: usize) -> &[u8] {
    debug_assert!(
        start_index
            .checked_add(count)
            .is_some_and(|end_index| end_index <= buffer.len()),
        "unchecked write range exceeds buffer"
    );
    // SAFETY: The caller guarantees that the computed pointer and length form
    // a valid subslice of `buffer`.
    unsafe { core::slice::from_raw_parts(buffer.as_ptr().add(start_index), count) }
}
