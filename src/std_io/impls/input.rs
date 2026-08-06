// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Standard [`Read`](std::io::Read) implementations of item input traits.

use std::io::{
    Read,
    Result,
};

use crate::util::UncheckedSlice;
use crate::{
    Input,
    traits::validate_read_count,
};

/// Bridges byte-oriented standard readers to item-oriented input.
impl<R> Input for R
where
    R: Read + ?Sized,
{
    /// Bytes read by the standard Read implementation.
    type Item = u8;

    /// Reads bytes into a caller-validated indexed output range.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Start offset inside `output`.
    /// - `count`: Maximum number of bytes to read.
    ///
    /// # Returns
    ///
    /// The number of bytes reported by the standard reader.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the standard reader.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested destination range does not fit.
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
        // SAFETY: The caller guarantees that the range is valid inside output.
        let target =
            unsafe { UncheckedSlice::subslice_mut(output, index, count) };
        Read::read(self, target)
    }

    /// Reads bytes into the complete output slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    ///
    /// # Returns
    ///
    /// The number of bytes read into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::ErrorKind::InvalidData`] if the reader reports more
    /// bytes than `output` holds, or the error reported by the standard reader.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        let read = Read::read(self, output)?;
        validate_read_count(read, output.len())?;
        Ok(read)
    }
}
