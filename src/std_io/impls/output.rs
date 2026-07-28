// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, Write};

use crate::{Output, UncheckedSlice, traits::validate_write_count};

/// Bridges byte-oriented standard writers to item-oriented output.
impl<W> Output for W
where
    W: Write + ?Sized,
{
    /// Bytes written by the standard Write implementation.
    type Item = u8;

    /// Writes bytes from a caller-validated indexed input range.
    ///
    /// Returns the error reported by the standard writer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that index through index plus count is a valid
    /// range inside input without overflowing.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), index, count),
            "unchecked write range exceeds input buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside input.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        Write::write(self, source)
    }

    /// Writes bytes from the complete input slice.
    ///
    /// Returns InvalidData if the writer reports more bytes than input contains.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        let written = Write::write(self, input)?;
        validate_write_count(written, input.len())?;
        Ok(written)
    }

    /// Flushes the standard writer.
    ///
    /// Returns the error reported by the standard writer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self)
    }
}
