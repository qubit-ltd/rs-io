// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::hash::Hasher;
use std::io::{Read, Result, Seek, SeekFrom};

/// Reader wrapper that updates a checksum hasher with bytes read.
///
/// `ChecksumReader` forwards reads to the wrapped reader and writes every
/// successfully read byte into the wrapped [`Hasher`]. Failed reads do not
/// update the hasher.
///
/// The checksum value is whatever the supplied [`Hasher`] reports. The Rust
/// standard-library hashers are not specified as stable file formats and are
/// not cryptographic digests; use this wrapper for stream instrumentation
/// unless the chosen hasher explicitly documents stronger guarantees.
///
/// Seeking changes only the wrapped reader position. It does not rewind,
/// subtract from, or otherwise adjust the hasher state.
///
/// `ChecksumReader` intentionally does not implement [`std::io::BufRead`].
/// A buffered-read implementation would have to choose whether bytes are hashed
/// when they are exposed by `fill_buf` or when the caller later consumes them;
/// that timing is easy to misread and differs from this type's `Read`
/// semantics, where only bytes returned by a successful read update the hasher.
///
/// If buffered access is needed, wrap this reader outside the checksum layer,
/// for example `BufReader<ChecksumReader<R, H>>`. In that composition bytes are
/// hashed when the outer `BufReader` refills its internal buffer, which may be
/// earlier than the application later consuming those bytes from `fill_buf`.
///
/// # Examples
/// ```
/// use std::collections::hash_map::DefaultHasher;
/// use std::hash::Hasher;
/// use std::io::{
///     Cursor,
///     Read,
/// };
///
/// use qubit_io::ChecksumReader;
///
/// let mut expected = DefaultHasher::new();
/// expected.write(b"payload");
///
/// let mut reader = ChecksumReader::new(Cursor::new(b"payload"), DefaultHasher::new());
/// let mut data = Vec::new();
/// reader.read_to_end(&mut data)?;
///
/// assert_eq!(b"payload", data.as_slice());
/// assert_eq!(expected.finish(), reader.checksum());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct ChecksumReader<R, H> {
    inner: R,
    hasher: H,
}

impl<R, H> ChecksumReader<R, H>
where
    H: Hasher,
{
    /// Creates a checksum reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `hasher`: Hasher updated with successfully read bytes.
    ///
    /// # Returns
    /// A new checksum reader.
    #[inline]
    pub fn new(inner: R, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    ///
    /// # Returns
    /// The value reported by [`Hasher::finish`].
    #[inline]
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
    }

    /// Returns an immutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline]
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns an immutable reference to the wrapped hasher.
    ///
    /// # Returns
    /// The wrapped hasher reference.
    #[inline]
    pub fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Returns a mutable reference to the wrapped hasher.
    ///
    /// # Returns
    /// The wrapped hasher reference.
    #[inline]
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns the wrapped reader and hasher.
    ///
    /// # Returns
    /// A tuple containing the wrapped reader and hasher.
    #[inline]
    pub fn into_inner(self) -> (R, H) {
        (self.inner, self.hasher)
    }
}

impl<R, H> Read for ChecksumReader<R, H>
where
    R: Read,
    H: Hasher,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buffer)?;
        self.hasher.write(&buffer[..count]);
        Ok(count)
    }
}

impl<R, H> Seek for ChecksumReader<R, H>
where
    R: Seek,
    H: Hasher,
{
    /// Seeks the wrapped reader without changing the hasher state.
    ///
    /// # Parameters
    /// - `position`: Target reader position.
    ///
    /// # Returns
    /// The new reader position.
    ///
    /// # Errors
    /// Returns the seek error reported by the wrapped reader.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
