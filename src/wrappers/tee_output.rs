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
use crate::traits::validate_write_count;
use crate::wrappers::internal::tee_failure::TeeFailure;

/// Output wrapper that mirrors successfully accepted items to a branch output.
///
/// Operations are ordered and not transactional. A branch write failure is
/// latched after primary progress; the write reports that progress, and later
/// non-empty writes, flushes, and seeks return the stored error. Check `flush`
/// after finite writes or inspect the error from
/// [`into_parts`](Self::into_parts). A returned `Ok(n)` describes primary
/// progress, not successful mirroring.
///
/// # Type Parameters
///
/// * `P` - Primary output type.
/// * `B` - Branch output type receiving mirrored items.
///
/// # Examples
///
/// ```
/// use qubit_io::Output;
/// use qubit_io::TeeOutput;
///
/// let primary = std::io::Cursor::new(Vec::new());
/// let branch = std::io::Cursor::new(Vec::new());
/// let mut output = TeeOutput::new(primary, branch);
///
/// assert_eq!(output.write(b"abc").unwrap(), 3);
/// assert_eq!(output.branch().get_ref(), b"abc");
/// ```
#[must_use]
#[derive(Debug)]
pub struct TeeOutput<P, B> {
    /// Primary output.
    inner: P,
    /// Branch output receiving mirrored items.
    branch: B,
    /// First branch write failure retained for later reporting.
    failure: TeeFailure,
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
        Self {
            inner,
            branch,
            failure: TeeFailure::new(),
        }
    }

    /// Returns a shared reference to the primary output.
    ///
    /// # Returns
    ///
    /// Returns the primary output without allowing mutation.
    #[inline(always)]
    #[must_use = "the primary output should be inspected or used"]
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
    #[must_use = "the primary output should be used or updated"]
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Returns a shared reference to the branch output.
    ///
    /// # Returns
    ///
    /// Returns the branch output without allowing mutation.
    #[inline(always)]
    #[must_use = "the branch output should be inspected or used"]
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
    #[must_use = "the branch output should be used or updated"]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Returns `Some(error)` after the first branch write failure, or `None`
    /// while mirroring has not failed.
    ///
    /// This state does not cover panics or arbitrary effects of custom streams.
    ///
    /// # Returns
    ///
    /// Returns the first recorded branch error, or `None` if mirroring has not
    /// failed.
    #[inline]
    pub fn branch_error(&self) -> Option<&io::Error> {
        self.failure.error()
    }

    /// Consumes this wrapper without flushing either output.
    ///
    /// This method performs no I/O. Any buffering owned by the returned primary
    /// or branch output remains pending and unchanged.
    ///
    /// # Returns
    ///
    /// Returns the primary output, branch output, and first branch error.
    #[inline(always)]
    #[must_use = "the outputs and first branch error should be handled"]
    pub fn into_parts(self) -> (P, B, Option<io::Error>) {
        (self.inner, self.branch, self.failure.into_error())
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
    /// Returns the number of items accepted by the primary output.
    ///
    /// # Errors
    ///
    /// Returns primary progress even if mirroring fails. The first branch
    /// error is latched and prevents later non-empty writes; callers must check
    /// [`branch_error`](Self::branch_error) or `flush` after finite writes.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] before the
    /// branch is written.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `input`.
    #[inline]
    unsafe fn write_unchecked(&mut self, input: &[Self::Item], index: usize, count: usize) -> io::Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        self.failure.check()?;
        let written = self.inner.write(&input[index..index + count])?;
        validate_write_count(written, count)?;
        if written != 0
            && let Err(error) = self.branch.write_fully(&input[index..index + written])
        {
            self.failure.record(error);
        }
        Ok(written)
    }

    /// Reports a latched branch error, or flushes the primary and then the
    /// branch.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after both outputs are flushed.
    ///
    /// # Errors
    ///
    /// Returns a previously latched branch write error without flushing either
    /// output. The primary may still contain buffered items; use
    /// [`into_parts`](Self::into_parts) to recover the outputs and inspect
    /// their state before deciding how to continue. Do not blindly replay
    /// items already accepted by the primary.
    ///
    /// If no branch write error is latched, returns the primary flush error
    /// without flushing the branch, or returns the branch flush error after the
    /// primary has flushed successfully.
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.failure.check()?;
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
    ///
    /// A branch seek error is returned directly and is not latched in
    /// `branch_error()`. Later transfers can still succeed at the streams'
    /// different positions. Stop ordinary transfers after such an error.
    /// Once the underlying seek failure is resolved, explicitly seek both
    /// streams through this wrapper to a chosen absolute position and check
    /// success before resuming. This applies only when no earlier
    /// mirror-transfer failure is latched. Repositioning aligns future
    /// transfers; it does not repair bytes already mirrored at an incorrect
    /// offset or roll back effects.
    #[inline]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.failure.check()?;
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
