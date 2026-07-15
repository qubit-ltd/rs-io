// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::hash::Hasher;
use std::io::{Result, Seek, SeekFrom, Write};

/// Writer wrapper that updates a checksum hasher with bytes written.
///
/// The wrapped hasher is updated only after the inner writer accepts bytes.
/// Failed writes do not update the hasher.
///
/// The checksum value is whatever the supplied [`Hasher`] reports. The Rust
/// standard-library hashers are not specified as stable file formats and are
/// not cryptographic digests; use this wrapper for stream instrumentation
/// unless the chosen hasher explicitly documents stronger guarantees.
///
/// Seeking changes only the wrapped writer position. It does not rewind,
/// subtract from, or otherwise adjust the hasher state.
///
/// # Examples
/// ```
/// use std::collections::hash_map::DefaultHasher;
/// use std::hash::Hasher;
/// use std::io::Write;
///
/// use qubit_io::ChecksumWriter;
///
/// let mut expected = DefaultHasher::new();
/// expected.write(b"payload");
///
/// let mut writer = ChecksumWriter::new(Vec::new(), DefaultHasher::new());
/// writer.write_all(b"payload")?;
///
/// assert_eq!(expected.finish(), writer.checksum());
/// assert_eq!(b"payload", writer.inner().as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct ChecksumWriter<W, H> {
    inner: W,
    hasher: H,
}

impl<W, H> ChecksumWriter<W, H>
where
    H: Hasher,
{
    /// Creates a checksum writer.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    /// - `hasher`: Hasher updated with successfully written bytes.
    ///
    /// # Returns
    /// A new checksum writer.
    #[inline]
    pub fn new(inner: W, hasher: H) -> Self {
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

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut W {
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

    /// Consumes this wrapper and returns the wrapped writer and hasher.
    ///
    /// # Returns
    /// A tuple containing the wrapped writer and hasher.
    #[inline]
    pub fn into_inner(self) -> (W, H) {
        (self.inner, self.hasher)
    }
}

impl<W, H> Write for ChecksumWriter<W, H>
where
    W: Write,
    H: Hasher,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        let count = self.inner.write(buffer)?;
        self.hasher.write(&buffer[..count]);
        Ok(count)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W, H> Seek for ChecksumWriter<W, H>
where
    W: Seek,
    H: Hasher,
{
    /// Seeks the wrapped writer without changing the hasher state.
    ///
    /// # Parameters
    /// - `position`: Target writer position.
    ///
    /// # Returns
    /// The new writer position.
    ///
    /// # Errors
    /// Returns the seek error reported by the wrapped writer.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
