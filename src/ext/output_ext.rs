// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Result};

use crate::Output;
use crate::util::UncheckedSlice;

/// Extension methods for [`Output`] values.
///
/// `OutputExt` keeps complete-write helpers outside the minimal [`Output`]
/// trait. The methods are implemented for every item-oriented output,
/// including `dyn Output` trait objects.
pub trait OutputExt: Output {
    /// Writes all items from the full input slice.
    ///
    /// This method repeatedly calls [`Output::write_unchecked`] until the full
    /// slice is accepted. Interrupted writes are retried.
    ///
    /// # Parameters
    /// - `input`: Source items.
    ///
    /// # Errors
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::WriteZero`] if the output accepts zero items before the
    /// slice is complete. Returns [`ErrorKind::InvalidData`] if the output
    /// reports accepting more items than requested.
    #[inline(always)]
    fn write_all(&mut self, input: &[Self::Item]) -> Result<()> {
        // SAFETY: The full input slice is a valid source range.
        unsafe { self.write_all_unchecked(input, 0, input.len()) }
    }

    /// Writes all items from an indexed input range without checking the range.
    ///
    /// This method repeatedly calls [`Output::write_unchecked`] until all
    /// `count` items are accepted. Interrupted writes are retried. A zero
    /// progress report before the range is complete is converted to
    /// [`ErrorKind::WriteZero`].
    ///
    /// # Parameters
    /// - `input`: Source storage.
    /// - `index`: Start index inside `input`.
    /// - `count`: Number of items to write.
    ///
    /// # Errors
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::WriteZero`] if the implementation accepts zero items before
    /// the requested range is complete. Returns [`ErrorKind::InvalidData`] if
    /// the implementation reports accepting more items than requested.
    ///
    /// # Safety
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn write_all_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), index, count),
            "unchecked write-all range exceeds input buffer"
        );
        let mut written = 0;
        while written < count {
            let remaining = count - written;
            // SAFETY: The caller guarantees the original source range is
            // valid; `written < count`, so this suffix remains inside it.
            match unsafe { self.write_unchecked(input, index + written, remaining) } {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole output range",
                    ));
                }
                Ok(progress) => {
                    if progress > remaining {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "writer reported {progress} items for a {remaining}-item range"
                            ),
                        ));
                    }
                    written += progress;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

impl<T> OutputExt for T where T: Output + ?Sized {}
