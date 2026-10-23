// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Standard [`Read`](std::io::Read) implementations of item input traits.

use std::io::Read;
use std::io::Result;

use crate::Input;
use crate::traits::validate_read_count;
use crate::util::UncheckedSlice;

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
    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> Result<usize> {
        // SAFETY: The caller guarantees that the range is valid inside output.
        let target = unsafe { UncheckedSlice::subslice_mut(output, index, count) };
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

    /// Reads from the buffers in order using the standard reader's vectored
    /// method. Returns `InvalidInput` if their total length overflows,
    /// `OutOfMemory` if descriptor preparation fails before I/O, or
    /// `InvalidData` if the reader reports more than the total capacity. Short
    /// progress and reader errors are returned without retrying or undoing
    /// underlying side effects.
    fn read_vectored(&mut self, buffers: &mut [&mut [u8]]) -> Result<usize> {
        let capacity = crate::util::requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let mut slices = crate::util::io_slices_mut(buffers)?;
        let count = Read::read_vectored(self, &mut slices)?;
        validate_read_count(count, capacity)?;
        Ok(count)
    }
}
