// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, SeekFrom};

use crate::{BufferedInput, Input, Seekable, SeekableInput};

/// Input value that is guaranteed to report itself as buffered.
///
/// This enum is returned by [`BufferedInput::ensure`]. It keeps an input that
/// already buffers items as-is, and wraps an unbuffered input in
/// [`BufferedInput`]. Operations delegate to the selected branch.
pub enum EnsuredBufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// The original input already reported itself as buffered.
    AlreadyBuffered(I),

    /// The original input was wrapped in [`BufferedInput`].
    Buffered(BufferedInput<I>),
}

impl<I> Input for EnsuredBufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    type Item = I::Item;

    /// Reports that this input is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Reads items from the selected input branch.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(input) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { input.read_unchecked(output, index, count) }
            }
            Self::Buffered(input) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { input.read_unchecked(output, index, count) }
            }
        }
    }

    /// Reads items into the full output slice.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(input) => input.read(output),
            Self::Buffered(input) => input.read(output),
        }
    }

    /// Reads items until the indexed output range is full or EOF is reached.
    #[inline(always)]
    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(input) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { input.read_fully_unchecked(output, index, count) }
            }
            Self::Buffered(input) => {
                // SAFETY: Forwarded from the trait caller.
                unsafe { input.read_fully_unchecked(output, index, count) }
            }
        }
    }

    /// Reads items into the full output slice until it is full or EOF is
    /// reached.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(input) => input.read_fully(output),
            Self::Buffered(input) => input.read_fully(output),
        }
    }
}

impl<I> Seekable for EnsuredBufferedInput<I>
where
    I: SeekableInput,
    <I as Input>::Item: Copy + Default,
{
    type Unit = <I as Input>::Item;

    /// Seeks in item offsets through the selected input branch.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        match self {
            Self::AlreadyBuffered(input) => input.seek_to(position),
            Self::Buffered(input) => input.seek_to(position),
        }
    }
}
