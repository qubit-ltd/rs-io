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

/// Tee input whose branch position follows every source seek.
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
    /// Returns the number of source items read and mirrored.
    ///
    /// # Errors
    ///
    /// Returns a source error before touching the branch. If mirroring fails,
    /// returns the branch error after the source has advanced and the
    /// destination has been modified.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] before the
    /// branch is written.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    #[inline]
    unsafe fn read_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> io::Result<usize> {
        let read = self.inner.read(&mut output[index..index + count])?;
        validate_read_count(read, count)?;
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
    #[inline]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek_to(position)?;
        self.branch.seek_to(SeekFrom::Start(position))?;
        Ok(position)
    }
}
