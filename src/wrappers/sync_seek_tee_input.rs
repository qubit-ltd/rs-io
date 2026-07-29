// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
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
///
/// Operations are ordered and not transactional. A branch write or seek
/// failure can leave the source advanced while the branch is behind.
///
/// # Type Parameters
///
/// * `I` - Source input type.
/// * `B` - Seekable branch output receiving mirrored items.
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
    ///
    /// # Parameters
    ///
    /// - `inner`: Source input.
    /// - `branch`: Seekable output that receives mirrored items.
    ///
    /// # Returns
    ///
    /// Returns a wrapper that keeps the branch position synchronized on seeks.
    #[inline(always)]
    pub const fn new(inner: I, branch: B) -> Self {
        Self { inner, branch }
    }

    /// Returns a shared reference to the source input.
    ///
    /// # Returns
    ///
    /// Returns the source input without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the source input.
    ///
    /// Direct reads and seeks bypass branch synchronization.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the source input.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a shared reference to the branch output.
    ///
    /// # Returns
    ///
    /// Returns the branch output without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn branch(&self) -> &B {
        &self.branch
    }

    /// Returns mutable access to the branch output.
    ///
    /// Direct operations can make the branch diverge from the source.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the branch output.
    #[inline(always)]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns its source and branch.
    ///
    /// # Returns
    ///
    /// Returns the source input and branch output.
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
    /// # Errors
    ///
    /// Returns a source error before touching the branch. If mirroring fails,
    /// returns the branch error after the source has advanced and the
    /// destination has been modified.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    #[inline(always)]
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
    ///
    /// # Errors
    ///
    /// Returns a source seek error without seeking the branch. If the branch
    /// seek fails, the source remains at its new position.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
