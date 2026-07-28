// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Read, Result};

use crate::{Input, UncheckedSlice, traits::validate_read_count};

/// Bridges byte-oriented standard readers to item-oriented input.
impl<R> Input for R
where
    R: Read + ?Sized,
{
    /// Bytes read by the standard Read implementation.
    type Item = u8;

    /// Reads bytes into a caller-validated indexed output range.
    ///
    /// Returns the error reported by the standard reader.
    ///
    /// # Safety
    ///
    /// The caller must ensure that index through index plus count is a valid
    /// range inside output without overflowing.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked read range exceeds output buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside output.
        let target = unsafe { UncheckedSlice::subslice_mut(output, index, count) };
        Read::read(self, target)
    }

    /// Reads bytes into the complete output slice.
    ///
    /// Returns InvalidData if the reader reports more bytes than output holds.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        let read = Read::read(self, output)?;
        validate_read_count(read, output.len())?;
        Ok(read)
    }
}
