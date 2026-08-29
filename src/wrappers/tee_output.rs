// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use crate::Output;
use crate::Seekable;

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
    ///
    /// # Parameters
    ///
    /// - `inner`: Primary output.
    /// - `branch`: Output that receives mirrored items.
    ///
    /// # Returns
    ///
    /// Returns a wrapper that mirrors successfully accepted items.
    #[inline(always)]
    pub const fn new(inner: P, branch: B) -> Self {
        Self { inner, branch }
    }

    /// Returns a shared reference to the primary output.
    ///
    /// # Returns
    ///
    /// Returns the primary output without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &P {
        &self.inner
    }

    /// Returns mutable access to the primary output.
    ///
    /// Direct writes, flushes, and seeks bypass the branch.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the primary output.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut P {
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
    /// Direct operations can make the branch diverge from the primary output.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the branch output.
    #[inline(always)]
    #[must_use]
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
    ///
    /// # Returns
    ///
    /// Returns `true` only when both outputs report themselves as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered() && self.branch.is_buffered()
    }

    /// Writes to the primary output and mirrors its successful item prefix.
    ///
    /// # Parameters
    ///
    /// - `input`: Source item slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns the number of primary items written and mirrored.
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
    #[inline]
    unsafe fn write_unchecked(&mut self, input: &[Self::Item], index: usize, count: usize) -> io::Result<usize> {
        let written = self.inner.write(&input[index..index + count])?;
        self.branch.write_fully(&input[index..index + written])?;
        Ok(written)
    }

    /// Flushes the primary output and then the branch output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after both outputs are flushed.
    ///
    /// # Errors
    ///
    /// Returns the primary flush error without flushing the branch, or returns
    /// the branch flush error after the primary has flushed successfully.
    #[inline]
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
    /// # Parameters
    ///
    /// - `position`: Target primary-output position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute position after both outputs are aligned.
    ///
    /// # Errors
    ///
    /// Returns a primary seek error without seeking the branch. If the branch
    /// seek fails, the primary remains at its new position.
    #[inline]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
