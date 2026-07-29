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
    Output,
    Seekable,
};

/// Output wrapper that mirrors successfully accepted items to a branch output.
///
/// Operations are ordered and not transactional. A branch write or seek
/// failure can leave the primary output advanced while the branch is behind.
///
/// # Type Parameters
///
/// * `P` - Primary output type.
/// * `B` - Branch output type receiving mirrored items.
#[must_use]
#[derive(Debug)]
pub struct TeeOutput<P, B> {
    /// Primary output.
    inner: P,
    /// Branch output receiving mirrored items.
    branch: B,
}

impl<P, B> TeeOutput<P, B> {
    /// Creates a tee output around `inner` and `branch`.
    #[inline(always)]
    pub const fn new(inner: P, branch: B) -> Self {
        Self { inner, branch }
    }

    /// Returns a shared reference to the primary output.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &P {
        &self.inner
    }

    /// Returns mutable access to the primary output.
    ///
    /// Direct writes, flushes, and seeks bypass the branch.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut P {
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
    /// Direct operations can make the branch diverge from the primary output.
    #[inline(always)]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper without flushing either output.
    ///
    /// This method performs no I/O. Any buffering owned by the returned primary
    /// or branch output remains pending and unchanged.
    ///
    /// # Returns
    ///
    /// Returns the primary output and branch output.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (P, B) {
        (self.inner, self.branch)
    }
}

impl<P, B> Output for TeeOutput<P, B>
where
    P: Output,
    B: Output<Item = P::Item>,
{
    /// Item type accepted by the primary output.
    type Item = P::Item;

    /// Returns true only when both output paths are buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered() && self.branch.is_buffered()
    }

    /// Writes to the primary output and mirrors its successful item prefix.
    ///
    /// # Errors
    ///
    /// Returns a primary error before touching the branch. If mirroring fails,
    /// returns the branch error after the primary has accepted the reported
    /// prefix.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `input`.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let written = self.inner.write(&input[index..index + count])?;
        self.branch.write_fully(&input[index..index + written])?;
        Ok(written)
    }

    /// Flushes the primary output and then the branch output.
    ///
    /// # Errors
    ///
    /// Returns the primary flush error without flushing the branch, or returns
    /// the branch flush error after the primary has flushed successfully.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        self.branch.flush()
    }
}

impl<P, B> Seekable for TeeOutput<P, B>
where
    P: Seekable,
    B: Seekable<Unit = P::Unit>,
{
    /// Unit used by both output positions.
    type Unit = P::Unit;

    /// Seeks the primary output and then aligns the branch output.
    ///
    /// # Errors
    ///
    /// Returns a primary seek error without seeking the branch. If the branch
    /// seek fails, the primary remains at its new position.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
