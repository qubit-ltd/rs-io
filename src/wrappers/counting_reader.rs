// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    BufRead,
    Read,
    Result,
    Seek,
    SeekFrom,
};

/// Reader wrapper that counts successfully read bytes.
///
/// The count is increased only after the wrapped reader reports a successful
/// byte count. Failed reads do not change the counter.
///
/// # Examples
/// ```
/// use std::io::{
///     Cursor,
///     Read,
/// };
///
/// use qubit_io::CountingReader;
///
/// let mut reader = CountingReader::new(Cursor::new(b"abc"));
/// let mut data = Vec::new();
/// reader.read_to_end(&mut data)?;
///
/// assert_eq!(b"abc", data.as_slice());
/// assert_eq!(3, reader.bytes_read());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Type Parameters
///
/// - `R`: Wrapped reader type.
#[must_use]
pub struct CountingReader<R> {
    /// Reader whose successful reads are counted.
    inner: R,
    /// Saturating count of returned bytes.
    bytes_read: u64,
}

impl<R> CountingReader<R> {
    /// Creates a counting reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    ///
    /// # Returns
    /// A new counting reader with a zero byte count.
    #[inline(always)]
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            bytes_read: 0,
        }
    }

    /// Returns the number of bytes successfully read through this wrapper.
    ///
    /// # Returns
    /// Total byte count. The value saturates at [`u64::MAX`].
    #[inline(always)]
    #[must_use]
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Returns an immutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline(always)]
    #[must_use]
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// Reads performed directly on the returned reader are not included in
    /// this wrapper's byte count.
    ///
    /// # Returns
    /// The wrapped reader reference.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> Read for CountingReader<R>
where
    R: Read,
{
    /// Reads bytes and counts only the successfully returned prefix.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Destination byte slice.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read and added to the saturating count.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped reader without changing the
    /// count.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buffer)?;
        self.bytes_read = self.bytes_read.saturating_add(count as u64);
        Ok(count)
    }
}

impl<R> BufRead for CountingReader<R>
where
    R: BufRead,
{
    /// Returns the wrapped reader's currently buffered bytes.
    ///
    /// # Returns
    ///
    /// Returns the byte slice exposed by the wrapped reader.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped reader.
    #[inline(always)]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_buf()
    }

    /// Consumes buffered bytes and adds them to the saturating count.
    ///
    /// # Parameters
    ///
    /// - `amount`: Number of previously exposed bytes to consume.
    #[inline]
    fn consume(&mut self, amount: usize) {
        self.bytes_read = self.bytes_read.saturating_add(amount as u64);
        self.inner.consume(amount);
    }
}

impl<R> Seek for CountingReader<R>
where
    R: Seek,
{
    /// Seeks the wrapped reader without changing the byte count.
    ///
    /// # Parameters
    ///
    /// - `position`: Target reader position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute position.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped reader.
    #[inline(always)]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
