// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    Read,
    Result,
    Seek,
    SeekFrom,
    Write,
};

use super::SyncSeekTeeReader;

/// Reader wrapper that mirrors read bytes into a branch writer.
///
/// `TeeReader` forwards reads to the source reader and writes every
/// successfully read byte into the branch writer while the stream is consumed.
/// If the branch writer fails, the source bytes have already been read from the
/// inner reader and the branch error is returned.
///
/// Seeking a `TeeReader` seeks only the source reader. It does not seek or
/// otherwise modify the branch writer; bytes mirrored after the seek are simply
/// appended or written according to the branch writer's own state.
///
/// `TeeReader` intentionally does not implement [`std::io::BufRead`]. Mirroring
/// from `fill_buf` would copy bytes before the caller commits to consuming
/// them, while mirroring from `consume` could not report branch write failures
/// because `BufRead::consume` has no error return.
///
/// If buffered access is needed, wrap this reader outside the tee layer, for
/// example `BufReader<TeeReader<R, W>>`. In that composition bytes are mirrored
/// when the outer `BufReader` refills its internal buffer, which may be earlier
/// than the application later consuming those bytes from `fill_buf`.
///
/// # Failure and retry semantics
///
/// A branch error does not restore bytes already consumed from the source.
/// Callers should therefore treat a branch write error as terminal unless they
/// can reposition or otherwise recover the source. An outer buffering layer
/// may discard a failed refill even though the source has already advanced:
///
/// ```text
/// source before refill: "abcdef"
/// refill reads "abc":   source advances to "def", branch returns an error
/// retry:                reading resumes at "def"; "abc" may be unavailable
/// ```
///
/// After a branch error, the source and branch may be out of sync. This wrapper
/// does not retain the consumed bytes or provide rollback.
///
/// # Examples
/// ```
/// use std::io::{
///     Cursor,
///     Read,
/// };
///
/// use qubit_io::TeeReader;
///
/// let source = Cursor::new(b"abc".to_vec());
/// let branch = Vec::new();
/// let mut reader = TeeReader::new(source, branch);
///
/// let mut data = Vec::new();
/// reader.read_to_end(&mut data)?;
/// let (_source, branch) = reader.into_inner();
///
/// assert_eq!(b"abc", data.as_slice());
/// assert_eq!(b"abc", branch.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct TeeReader<R, W> {
    inner: R,
    branch: W,
}

impl<R, W> TeeReader<R, W> {
    /// Creates a tee reader.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `branch`: Writer that receives the bytes successfully read.
    ///
    /// # Returns
    /// A new tee reader.
    #[inline]
    pub fn new(reader: R, branch: W) -> Self {
        Self {
            inner: reader,
            branch,
        }
    }

    /// Creates a tee reader that synchronizes branch seeks with source seeks.
    ///
    /// The returned wrapper mirrors read bytes like [`TeeReader`], but its
    /// [`Seek`] implementation also seeks the branch writer to the source
    /// reader's resulting absolute position. If the branch seek fails, the
    /// source reader may already have moved.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `branch`: Writer that receives bytes successfully read and is sought
    ///   to the same absolute position as the source reader.
    ///
    /// # Returns
    /// A sync-seek tee reader.
    #[inline]
    pub fn with_sync_branch_seek(
        reader: R,
        branch: W,
    ) -> SyncSeekTeeReader<R, W> {
        SyncSeekTeeReader::new(reader, branch)
    }

    /// Returns an immutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    #[inline]
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch(&self) -> &W {
        &self.branch
    }

    /// Returns a mutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch_mut(&mut self) -> &mut W {
        &mut self.branch
    }

    /// Consumes this wrapper and returns the source reader and branch writer.
    ///
    /// # Returns
    /// A tuple containing the source reader and branch writer.
    #[inline]
    pub fn into_inner(self) -> (R, W) {
        (self.inner, self.branch)
    }
}

impl<R, W> Read for TeeReader<R, W>
where
    R: Read,
    W: Write,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buffer)?;
        self.branch.write_all(&buffer[..count])?;
        Ok(count)
    }
}

impl<R, W> Seek for TeeReader<R, W>
where
    R: Seek,
{
    /// Seeks the source reader without touching the branch writer.
    ///
    /// # Parameters
    /// - `position`: Target position for the source reader.
    ///
    /// # Returns
    /// The new source reader position.
    ///
    /// # Errors
    /// Returns the seek error reported by the source reader.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
