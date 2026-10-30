// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Standard [`Write`](std::io::Write) implementations of item output traits.

use std::io::Result;
use std::io::Write;

use crate::Output;
use crate::traits::validate_write_count;
use crate::util::UncheckedSlice;

/// Bridges byte-oriented standard writers to item-oriented output.
impl<W> Output for W
where
    W: Write + ?Sized,
{
    /// Bytes written by the standard Write implementation.
    type Item = u8;

    /// Writes bytes from a caller-validated indexed input range.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Start offset inside `input`.
    /// - `count`: Maximum number of bytes to write.
    ///
    /// # Returns
    ///
    /// The number of bytes reported by the standard writer.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the standard writer.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested source range does not fit.
    ///
    /// # Safety
    ///
    /// The caller must ensure that index through index plus count is a valid
    /// range inside input without overflowing.
    #[inline(always)]
    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> Result<usize> {
        // SAFETY: The caller guarantees that the range is valid inside input.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        Write::write(self, source)
    }

    /// Writes bytes from the complete input slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    ///
    /// # Returns
    ///
    /// The number of bytes written from `input`.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::ErrorKind::InvalidData`] if the writer reports more
    /// bytes than `input` contains, or the error reported by the standard
    /// writer.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        let written = Write::write(self, input)?;
        validate_write_count(written, input.len())?;
        Ok(written)
    }

    /// Writes from the buffers in order using the standard writer's vectored
    /// method. A single descriptor uses stack storage; multiple descriptors
    /// may allocate descriptor storage, and the writer may allocate
    /// independently. Returns `InvalidInput` if their total length overflows,
    /// `OutOfMemory` if multiple-descriptor preparation fails before I/O, or
    /// `InvalidData` if the writer reports more than the total capacity. Short
    /// progress and writer errors are returned without retrying or undoing
    /// underlying side effects.
    fn write_vectored(&mut self, buffers: &[&[u8]]) -> Result<usize> {
        let capacity = crate::util::requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let count = if let [buffer] = buffers {
            let slices = [std::io::IoSlice::new(buffer)];
            Write::write_vectored(self, &slices)?
        } else {
            let slices = crate::util::io_slices(buffers)?;
            Write::write_vectored(self, &slices)?
        };
        validate_write_count(count, capacity)?;
        Ok(count)
    }

    /// Flushes the standard writer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the standard writer is flushed.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the standard writer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self)
    }
}
