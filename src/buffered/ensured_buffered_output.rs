// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, SeekFrom};

use crate::{BufferedOutput, Output, Seekable, SeekableOutput};

/// Output value that is guaranteed to report itself as buffered.
///
/// This enum is returned by [`BufferedOutput::ensure`]. It keeps an output that
/// already buffers items as-is, and wraps an unbuffered output in
/// [`BufferedOutput`]. Operations delegate to the selected branch.
///
/// # Type Parameters
///
/// - `O`: Output type that may require buffering.
#[must_use]
pub enum EnsuredBufferedOutput<O>
where
    O: Output,
    O::Item: Clone + Default,
{
    /// The original output already reported itself as buffered.
    AlreadyBuffered(
        /// Original buffered output.
        O,
    ),

    /// The original output was wrapped in [`BufferedOutput`].
    Buffered(
        /// Buffered wrapper around the original output.
        BufferedOutput<O>,
    ),
}

impl<O> Output for EnsuredBufferedOutput<O>
where
    O: Output,
    O::Item: Clone + Default,
{
    /// Item type accepted by the selected output.
    type Item = O::Item;

    /// Reports that this output is buffered.
    ///
    /// # Returns
    ///
    /// Always returns `true`.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Writes items through the selected output branch.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns the number of items written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline]
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(output) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { output.write_unchecked(input, index, count) }
            }
            Self::Buffered(output) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { output.write_unchecked(input, index, count) }
            }
        }
    }

    /// Writes items from the full input slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected output.
    #[inline]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(output) => output.write(input),
            Self::Buffered(output) => output.write(input),
        }
    }

    /// Writes all items from an indexed input range.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Number of items to write.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after every requested item is written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected output, including premature
    /// write-zero failures.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline]
    unsafe fn write_fully_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<()> {
        match self {
            Self::AlreadyBuffered(output) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { output.write_fully_unchecked(input, index, count) }
            }
            Self::Buffered(output) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { output.write_fully_unchecked(input, index, count) }
            }
        }
    }

    /// Writes all items from the full input slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after every item is written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected output, including premature
    /// write-zero failures.
    #[inline]
    fn write_fully(&mut self, input: &[Self::Item]) -> Result<()> {
        match self {
            Self::AlreadyBuffered(output) => output.write_fully(input),
            Self::Buffered(output) => output.write_fully(input),
        }
    }

    /// Flushes pending items through the selected output branch.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the selected output is flushed.
    ///
    /// # Errors
    ///
    /// Returns an error reported while flushing the selected output.
    #[inline]
    fn flush(&mut self) -> Result<()> {
        match self {
            Self::AlreadyBuffered(output) => output.flush(),
            Self::Buffered(output) => output.flush(),
        }
    }
}

impl<O> Seekable for EnsuredBufferedOutput<O>
where
    O: SeekableOutput,
    <O as Output>::Item: Clone + Default,
{
    /// Item unit used for seek offsets.
    type Unit = <O as Output>::Item;

    /// Seeks in item offsets through the selected output branch.
    ///
    /// # Parameters
    ///
    /// - `position`: Target item offset.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute item position.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected output.
    #[inline]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        match self {
            Self::AlreadyBuffered(output) => output.seek_to(position),
            Self::Buffered(output) => output.seek_to(position),
        }
    }
}
