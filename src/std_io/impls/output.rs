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
