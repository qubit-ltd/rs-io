// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::io::{
    self,
    SeekFrom,
};

use crate::{
    Input,
    Output,
    Seekable,
};

/// Tee input whose branch position follows every source seek.
#[must_use]
#[derive(Debug)]
pub struct SyncSeekTeeInput<I, B> {
    /// Source input.
    inner: I,
    /// Branch output receiving mirrored items.
    branch: B,
}

impl<I, B> SyncSeekTeeInput<I, B> {
    /// Creates a synchronized-seek tee input.
    #[inline(always)]
    pub const fn new(inner: I, branch: B) -> Self {
        Self { inner, branch }
    }

    /// Returns a shared reference to the source input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the source input.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a shared reference to the branch output.
    #[inline(always)]
    #[must_use]
    pub const fn branch(&self) -> &B {
        &self.branch
    }

    /// Returns mutable access to the branch output.
    #[inline(always)]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns its source and branch.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, B) {
        (self.inner, self.branch)
    }
}

impl<I, B> Input for SyncSeekTeeInput<I, B>
where
    I: Input,
    B: Output<Item = I::Item>,
{
    /// Item type returned by the source input.
    type Item = I::Item;

    /// Returns the source input's buffering declaration.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads from the source and mirrors the successful item prefix.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = self.inner.read(&mut output[index..index + count])?;
        self.branch.write_fully(&output[index..index + read])?;
        Ok(read)
    }
}

impl<I, B> Seekable for SyncSeekTeeInput<I, B>
where
    I: Seekable,
    B: Seekable<Unit = I::Unit>,
{
    /// Unit used by both stream positions.
    type Unit = I::Unit;

    /// Seeks the source and then aligns the branch output.
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
