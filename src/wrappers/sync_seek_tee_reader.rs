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

/// Reader wrapper that mirrors read bytes and keeps the branch seek position in
/// sync.
///
/// `SyncSeekTeeReader` has the same read behavior as [`crate::TeeReader`]:
/// bytes returned by the source reader are written to the branch writer. The
/// difference is seek behavior. Seeking moves the source reader first, then
/// seeks the branch writer to the source reader's resulting absolute position.
///
/// If the branch seek fails, the source reader may already have moved. If a
/// branch write fails during reading, the source bytes have already been read
/// and the branch error is returned.
///
/// # Examples
/// ```
/// use std::io::{
///     Cursor,
///     Read,
///     Seek,
///     SeekFrom,
/// };
///
/// use qubit_io::TeeReader;
///
/// let source = Cursor::new(b"abcdef".to_vec());
/// let branch = Cursor::new(vec![0; 6]);
/// let mut reader = TeeReader::with_sync_branch_seek(source, branch);
///
/// reader.seek(SeekFrom::Start(2))?;
/// let mut data = [0; 2];
/// reader.read_exact(&mut data)?;
///
/// assert_eq!(b"cd", &data);
/// assert_eq!(&[0, 0, b'c', b'd', 0, 0], reader.branch().get_ref().as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct SyncSeekTeeReader<R, W> {
    inner: R,
    branch: W,
}

impl<R, W> SyncSeekTeeReader<R, W> {
    /// Creates a tee reader that synchronizes branch seeks.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `branch`: Writer that receives bytes successfully read and is sought
    ///   to the same absolute position as the source reader.
    ///
    /// # Returns
    /// A new sync-seek tee reader.
    #[inline]
    pub fn new(reader: R, branch: W) -> Self {
        Self {
            inner: reader,
            branch,
        }
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
    /// Reads and seeks performed directly on the returned reader are not
    /// mirrored to the branch writer.
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
    /// Mutating the returned writer independently can desynchronize it from the
    /// source reader.
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

impl<R, W> Read for SyncSeekTeeReader<R, W>
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

impl<R, W> Seek for SyncSeekTeeReader<R, W>
where
    R: Seek,
    W: Seek,
{
    /// Seeks the source reader, then seeks the branch writer to the same
    /// position.
    ///
    /// # Parameters
    /// - `position`: Target position for the source reader.
    ///
    /// # Returns
    /// The new source reader position.
    ///
    /// # Errors
    /// Returns the source seek error, or the branch seek error after the source
    /// reader has already moved.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        let reader_position = self.inner.seek(position)?;
        self.branch.seek(SeekFrom::Start(reader_position))?;
        Ok(reader_position)
    }
}
