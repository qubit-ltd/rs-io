// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use crate::Input;
use crate::Output;
use crate::Seekable;
use crate::traits::validate_read_count;
use crate::wrappers::internal::tee_failure::TeeFailure;

/// Tee input whose branch position follows every source seek.
///
/// A branch write failure is latched after source progress. The read reports
/// that progress, and later non-empty reads and seeks return the stored error.
/// Check [`branch_error`](Self::branch_error) after finite reads or inspect the
/// error returned by [`into_parts`](Self::into_parts).
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use std::io::SeekFrom;
///
/// use qubit_io::Input;
/// use qubit_io::Seekable;
/// use qubit_io::SyncSeekTeeInput;
///
/// let mut input = SyncSeekTeeInput::new(Cursor::new(b"abc"), Cursor::new(Vec::new()));
/// let mut bytes = [0; 2];
/// assert_eq!(2, input.read(&mut bytes)?);
/// assert_eq!(&bytes, b"ab");
/// assert_eq!(1, input.seek_to(SeekFrom::Start(1))?);
/// assert_eq!(1, input.inner().position());
/// assert_eq!(1, input.branch().position());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// Construction does not align the branch with the source. Seek and mirror
/// operations are ordered but not transactional; a branch error can leave the
/// source advanced while the branch is behind.
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
    failure: TeeFailure,
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
        Self {
            inner,
            branch,
            failure: TeeFailure::new(),
        }
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
    #[must_use]
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
    #[must_use]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Returns `Some(error)` after the first branch write failure, or `None`
    /// while mirroring has not failed.
    ///
    /// This state does not cover panics or arbitrary effects of custom streams.
    pub fn branch_error(&self) -> Option<&io::Error> {
        self.failure.error()
    }

    /// Consumes this wrapper and returns its source and branch.
    ///
    /// # Returns
    ///
    /// Returns the source input, branch output, and first branch error.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, B, Option<io::Error>) {
        (self.inner, self.branch, self.failure.into_error())
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
    ///
    /// # Returns
    ///
    /// Returns whether the source input reports itself as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads from the source and mirrors the successful item prefix.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination item slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// Returns the number of items read from the primary source.
    ///
    /// # Errors
    ///
    /// Returns primary progress even if mirroring fails. The first branch
    /// error is latched and prevents later non-empty reads; inspect
    /// [`branch_error`](Self::branch_error) after finite reads.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] before the
    /// branch is written.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    #[inline]
    unsafe fn read_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> io::Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        self.failure.check()?;
        let read = self.inner.read(&mut output[index..index + count])?;
        validate_read_count(read, count)?;
        if read != 0
            && let Err(error) = self.branch.write_fully(&output[index..index + read])
        {
            self.failure.record(error);
        }
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
    /// # Parameters
    ///
    /// - `position`: Target source position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute position after both streams are aligned.
    ///
    /// # Errors
    ///
    /// Returns a source seek error without seeking the branch. If the branch
    /// seek fails, the source remains at its new position.
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
