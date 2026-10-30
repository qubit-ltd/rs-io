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
use crate::util::io_slices_mut;
use crate::util::requested_items;

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
    /// A successful call may read fewer than `count` bytes. A return value of
    /// zero indicates that no bytes were read; for a nonempty request this
    /// normally signals end of input. This method does not retry short reads.
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
    /// A successful call may fill only part of `output`; zero means no bytes
    /// were read and normally signals end of input for a nonempty buffer. This
    /// method does not retry short reads.
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
    /// method. A single descriptor uses stack storage; multiple descriptors
    /// may allocate descriptor storage, and the reader may allocate
    /// independently. Short progress and reader errors are returned without
    /// retrying or undoing underlying side effects.
    ///
    /// # Parameters
    ///
    /// - `buffers`: Mutable byte buffers to fill in order.
    ///
    /// # Returns
    ///
    /// The number of bytes reported by the standard reader, up to the total
    /// capacity of `buffers`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::ErrorKind::InvalidInput`] if the total buffer length
    /// overflows, [`std::io::ErrorKind::OutOfMemory`] if multiple-descriptor
    /// preparation fails before I/O, [`std::io::ErrorKind::InvalidData`] if
    /// the reader reports more than the total capacity, or the error reported
    /// by the standard reader.
    fn read_vectored(&mut self, buffers: &mut [&mut [u8]]) -> Result<usize> {
        let capacity = requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let count = if let [buffer] = buffers {
            let mut slices = [std::io::IoSliceMut::new(buffer)];
            Read::read_vectored(self, &mut slices)?
        } else {
            let mut slices = io_slices_mut(buffers)?;
            Read::read_vectored(self, &mut slices)?
        };
        validate_read_count(count, capacity)?;
        Ok(count)
    }
}
