// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Result,
    SeekFrom,
};

use crate::{
    BufferedInput,
    Input,
    Seekable,
    SeekableInput,
};

/// Input value that is guaranteed to report itself as buffered.
///
/// This enum is returned by [`BufferedInput::ensure`]. It keeps an input that
/// already buffers items as-is, and wraps an unbuffered input in
/// [`BufferedInput`]. Operations delegate to the selected branch.
///
/// # Type Parameters
///
/// - `I`: Input type that may require buffering.
#[must_use]
pub enum EnsuredBufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// The original input already reported itself as buffered.
    AlreadyBuffered(
        /// Original buffered input.
        I,
    ),

    /// The original input was wrapped in [`BufferedInput`].
    Buffered(
        /// Buffered wrapper around the original input.
        BufferedInput<I>,
    ),
}

impl<I> Input for EnsuredBufferedInput<I>
where
    I: Input,
    I::Item: Copy + Default,
{
    /// Item type produced by the selected input.
    type Item = I::Item;

    /// Reports that this input is buffered.
    ///
    /// # Returns
    ///
    /// Always returns `true`.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        true
    }

    /// Reads items from the selected input branch.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
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
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected input.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        match self {
            Self::AlreadyBuffered(input) => input.read(output),
            Self::Buffered(input) => input.read(output),
        }
    }

    /// Reads items until the indexed output range is full or EOF is reached.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Number of items requested.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the range filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected input.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `output`.
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
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items read before the slice filled or EOF was
    /// reached.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the selected input.
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
    /// Item unit used for seek offsets.
    type Unit = <I as Input>::Item;

    /// Seeks in item offsets through the selected input branch.
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
    /// Returns an error reported by the selected input.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        match self {
            Self::AlreadyBuffered(input) => input.seek_to(position),
            Self::Buffered(input) => input.seek_to(position),
        }
    }
}
