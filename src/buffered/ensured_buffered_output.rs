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
pub enum EnsuredBufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    /// The original output already reported itself as buffered.
    AlreadyBuffered(O),

    /// The original output was wrapped in [`BufferedOutput`].
    Buffered(BufferedOutput<O>),
}

impl<O> Output for EnsuredBufferedOutput<O>
where
    O: Output,
    O::Item: Copy + Default,
{
    type Item = O::Item;

    /// Reports that this output is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Writes items through the selected output branch.
    #[inline(always)]
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
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(output) => output.write(input),
            Self::Buffered(output) => output.write(input),
        }
    }

    /// Writes all items from an indexed input range.
    #[inline(always)]
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
    #[inline(always)]
    fn write_fully(&mut self, input: &[Self::Item]) -> Result<()> {
        match self {
            Self::AlreadyBuffered(output) => output.write_fully(input),
            Self::Buffered(output) => output.write_fully(input),
        }
    }

    /// Flushes pending items through the selected output branch.
    #[inline(always)]
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
    <O as Output>::Item: Copy + Default,
{
    type Unit = <O as Output>::Item;

    /// Seeks in item offsets through the selected output branch.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        match self {
            Self::AlreadyBuffered(output) => output.seek_to(position),
            Self::Buffered(output) => output.seek_to(position),
        }
    }
}
