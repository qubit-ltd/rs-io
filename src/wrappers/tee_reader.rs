// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{Read, Result, Seek, SeekFrom, Write};

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
/// from `fill_buf` would copy bytes before the caller commits to consuming them,
/// while mirroring from `consume` could not report branch write failures because
/// `BufRead::consume` has no error return.
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
    reader: R,
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
        Self { reader, branch }
    }

    /// Returns an immutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    #[inline]
    pub fn reader_ref(&self) -> &R {
        &self.reader
    }

    /// Returns a mutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    #[inline]
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch_ref(&self) -> &W {
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
        (self.reader, self.branch)
    }
}

impl<R, W> Read for TeeReader<R, W>
where
    R: Read,
    W: Write,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.reader.read(buffer)?;
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
        self.reader.seek(position)
    }
}
