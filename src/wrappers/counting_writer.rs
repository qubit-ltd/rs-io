/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{Result, Seek, SeekFrom, Write};

/// Writer wrapper that counts successfully written bytes.
///
/// The count is increased only after the wrapped writer reports a successful
/// byte count. Failed writes do not change the counter.
///
/// # Examples
/// ```
/// use std::io::Write;
///
/// use qubit_io::CountingWriter;
///
/// let mut writer = CountingWriter::new(Vec::new());
/// writer.write_all(b"abc")?;
///
/// assert_eq!(3, writer.bytes_written());
/// assert_eq!(b"abc", writer.get_ref().as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct CountingWriter<W> {
    inner: W,
    bytes_written: u64,
}

impl<W> CountingWriter<W> {
    /// Creates a counting writer.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    ///
    /// # Returns
    /// A new counting writer with a zero byte count.
    #[inline]
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }

    /// Returns the number of bytes successfully written through this wrapper.
    ///
    /// # Returns
    /// Total byte count. The value saturates at [`u64::MAX`].
    #[inline]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> Write for CountingWriter<W>
where
    W: Write,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        let count = self.inner.write(buffer)?;
        self.bytes_written = self.bytes_written.saturating_add(count as u64);
        Ok(count)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W> Seek for CountingWriter<W>
where
    W: Seek,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.inner.seek(position)
    }
}
