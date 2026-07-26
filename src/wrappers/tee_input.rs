// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{self, SeekFrom};

use crate::{Input, Output, Seekable, SyncSeekTeeInput};

/// Input wrapper that mirrors successfully returned items to a branch output.
///
/// A branch failure is not transactional: the source has already advanced and
/// the caller's destination already contains the returned items.
///
/// # Type Parameters
///
/// * `I` - Source input type.
/// * `B` - Branch output type receiving mirrored items.
#[must_use]
#[derive(Debug)]
pub struct TeeInput<I, B> {
    /// Source input.
    inner: I,
    /// Branch output receiving mirrored items.
    branch: B,
}

impl<I, B> TeeInput<I, B> {
    /// Creates a tee input around `inner` and `branch`.
    #[inline(always)]
    pub const fn new(inner: I, branch: B) -> Self {
        Self { inner, branch }
    }

    /// Creates a tee input whose branch position follows source seeks.
    #[inline(always)]
    pub const fn with_sync_branch_seek(inner: I, branch: B) -> SyncSeekTeeInput<I, B> {
        SyncSeekTeeInput::new(inner, branch)
    }

    /// Returns a shared reference to the source input.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the source input.
    ///
    /// Reads and seeks made through the returned reference bypass the branch.
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
    ///
    /// Direct writes can make branch content diverge from the source.
    #[inline(always)]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns the source input and branch output.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, B) {
        (self.inner, self.branch)
    }
}

impl<I, B> Input for TeeInput<I, B>
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

impl<I, B> Seekable for TeeInput<I, B>
where
    I: Seekable,
{
    /// Unit used by the source input for stream positions.
    type Unit = I::Unit;

    /// Seeks only the source input.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
