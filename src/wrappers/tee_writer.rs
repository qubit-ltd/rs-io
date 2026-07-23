// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    Result,
    Seek,
    SeekFrom,
    Write,
};

/// Writer wrapper that mirrors accepted bytes into a branch writer.
///
/// `TeeWriter` writes to the primary writer first, then writes exactly the
/// accepted bytes into the branch writer with [`Write::write_all`]. If the
/// branch writer fails, the primary writer may already have accepted bytes and
/// the branch error is returned.
///
/// Seeking moves the primary writer first, then seeks the branch writer to the
/// primary writer's resulting absolute position. If the branch seek fails, the
/// primary writer may already have moved.
///
/// # Failure and retry semantics
///
/// A branch error does not roll back bytes already accepted by the primary
/// writer. Callers should therefore treat a branch write error as terminal
/// unless repeating the primary write is known to be safe. In particular, an
/// outer buffering layer may retain the entire failed chunk because
/// [`Write::write`] returned an error rather than a byte count. Retrying that
/// buffered chunk can duplicate data in the primary writer:
///
/// ```text
/// first attempt: primary accepts "abc", branch returns an error
/// retry:         primary accepts "abc" again
/// result:        primary may contain "abcabc"
/// ```
///
/// After a branch error, the primary and branch writers may be out of sync.
/// This wrapper does not record a sticky error or provide rollback.
///
/// # Examples
/// ```
/// use std::io::Write;
///
/// use qubit_io::TeeWriter;
///
/// let primary = Vec::new();
/// let branch = Vec::new();
/// let mut writer = TeeWriter::new(primary, branch);
///
/// writer.write_all(b"abc")?;
/// let (primary, branch) = writer.into_inner();
///
/// assert_eq!(b"abc", primary.as_slice());
/// assert_eq!(b"abc", branch.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct TeeWriter<P, B> {
    inner: P,
    branch: B,
}

impl<P, B> TeeWriter<P, B> {
    /// Creates a tee writer.
    ///
    /// # Parameters
    /// - `primary`: Primary destination writer.
    /// - `branch`: Secondary writer that mirrors accepted bytes.
    ///
    /// # Returns
    /// A new tee writer.
    #[inline]
    pub fn new(primary: P, branch: B) -> Self {
        Self {
            inner: primary,
            branch,
        }
    }

    /// Returns an immutable reference to the primary writer.
    ///
    /// # Returns
    /// The primary writer reference.
    #[inline]
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Returns a mutable reference to the primary writer.
    ///
    /// Writes performed directly on the returned writer are not mirrored to the
    /// branch writer.
    ///
    /// # Returns
    /// The primary writer reference.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch(&self) -> &B {
        &self.branch
    }

    /// Returns a mutable reference to the branch writer.
    ///
    /// Writing directly to the returned branch can desynchronize it from the
    /// primary writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns both wrapped writers.
    ///
    /// # Returns
    /// A tuple containing the primary writer and branch writer.
    #[inline]
    pub fn into_inner(self) -> (P, B) {
        (self.inner, self.branch)
    }
}

impl<P, B> Write for TeeWriter<P, B>
where
    P: Write,
    B: Write,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        let count = self.inner.write(buffer)?;
        self.branch.write_all(&buffer[..count])?;
        Ok(count)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()?;
        self.branch.flush()
    }
}

impl<P, B> Seek for TeeWriter<P, B>
where
    P: Seek,
    B: Seek,
{
    /// Seeks both wrapped writers to the same resulting absolute position.
    ///
    /// # Parameters
    /// - `position`: Target position for the primary writer.
    ///
    /// # Returns
    /// The new primary writer position.
    ///
    /// # Errors
    /// Returns the primary seek error, or the branch seek error after the
    /// primary writer has already moved.
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        let primary_position = self.inner.seek(position)?;
        self.branch.seek(SeekFrom::Start(primary_position))?;
        Ok(primary_position)
    }
}
