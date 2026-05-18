/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    Read,
    Result,
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
pub struct CountingReader<R> {
    inner: R,
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
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Returns an immutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader reference.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped reader.
    ///
    /// # Returns
    /// The wrapped reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> Read for CountingReader<R>
where
    R: Read,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buffer)?;
        self.bytes_read = self.bytes_read.saturating_add(count as u64);
        Ok(count)
    }
}
