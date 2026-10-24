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
use crate::SyncSeekTeeInput;
use crate::traits::validate_read_count;
use crate::wrappers::internal::tee_failure::TeeFailure;

/// Input wrapper that mirrors successfully returned items to a branch output.
///
/// A branch failure is latched after the source and destination have advanced.
/// The read still reports primary progress; later non-empty reads fail before
/// touching either stream. Callers must inspect
/// [`branch_error`](Self::branch_error) after finite reads or the error
/// returned by [`into_parts`](Self::into_parts).
///
/// # Type Parameters
///
/// * `I` - Source input type.
/// * `B` - Branch output type receiving mirrored items.
///
/// # Examples
///
/// ```
/// use qubit_io::Input;
/// use qubit_io::TeeInput;
///
/// let source = std::io::Cursor::new(&b"abc"[..]);
/// let branch = std::io::Cursor::new(Vec::new());
/// let mut input = TeeInput::new(source, branch);
/// let mut buffer = [0; 3];
///
/// assert_eq!(input.read(&mut buffer).unwrap(), 3);
/// assert_eq!(buffer, *b"abc");
/// assert_eq!(input.branch().get_ref(), b"abc");
/// ```
#[must_use]
#[derive(Debug)]
pub struct TeeInput<I, B> {
    /// Source input.
    inner: I,
    /// Branch output receiving mirrored items.
    branch: B,
    failure: TeeFailure,
}

impl<I, B> TeeInput<I, B> {
    /// Creates a tee input around `inner` and `branch`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Source input.
    /// - `branch`: Output that receives mirrored items.
    ///
    /// # Returns
    ///
    /// Returns a wrapper that mirrors successfully returned items.
    #[inline(always)]
    pub const fn new(inner: I, branch: B) -> Self {
        Self {
            inner,
            branch,
            failure: TeeFailure::new(),
        }
    }

    /// Creates a tee input whose branch position follows source seeks.
    ///
    /// # Parameters
    ///
    /// - `inner`: Source input.
    /// - `branch`: Seekable output that receives mirrored items.
    ///
    /// # Returns
    ///
    /// Returns a synchronized-seek tee input.
    #[inline(always)]
    pub const fn with_sync_branch_seek(inner: I, branch: B) -> SyncSeekTeeInput<I, B> {
        SyncSeekTeeInput::new(inner, branch)
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
    /// Reads and seeks made through the returned reference bypass the branch.
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
    /// Direct writes can make branch content diverge from the source.
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

    /// Consumes this wrapper and returns both streams and any latched branch
    /// error.
    ///
    /// # Returns
    ///
    /// Returns the source input, branch output, and first branch error, if any.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, B, Option<io::Error>) {
        (self.inner, self.branch, self.failure.into_error())
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
    /// Returns the primary progress even if mirroring fails. The first branch
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

impl<I, B> Seekable for TeeInput<I, B>
where
    I: Seekable,
{
    /// Unit used by the source input for stream positions.
    type Unit = I::Unit;

    /// Seeks only the source input.
    ///
    /// # Parameters
    ///
    /// - `position`: Target source position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute source position.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the source input.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.failure.check()?;
        self.inner.seek_to(position)
    }
}
