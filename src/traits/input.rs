// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Read, Result};

use crate::util::UncheckedSlice;

/// Minimal indexed input interface over items.
///
/// `Input` is intentionally smaller and lower-level than [`Read`]. It only
/// states that an implementor can read up to `count` items into
/// `output[index..index + count]`. The caller owns range validation so hot
/// paths can avoid repeated slicing and bounds checks.
pub trait Input {
    /// The unit type read from this input.
    type Item;

    /// Reads items into an indexed output range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// The number of items written into `output[index..index + count]`. The
    /// value must be in `0..=count`.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_into(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize>;
}

impl<R> Input for R
where
    R: Read + ?Sized,
{
    type Item = u8;

    /// Reads bytes from a standard [`Read`] value into an indexed range.
    #[inline(always)]
    unsafe fn read_into(&mut self, output: &mut [u8], index: usize, count: usize) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked read range exceeds output buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `output`.
        let target = unsafe { UncheckedSlice::subslice_mut(output, index, count) };
        Read::read(self, target)
    }
}
