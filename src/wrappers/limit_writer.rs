// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    Result,
    Write,
};

/// Writer wrapper that accepts at most a fixed number of bytes.
///
/// Once the remaining limit reaches zero, writes return `Ok(0)` without
/// touching the inner writer. Callers using [`Write::write_all`] will therefore
/// receive the standard write-zero error when trying to write beyond the limit.
///
/// # Examples
/// ```
/// use std::io::Write;
///
/// use qubit_io::LimitWriter;
///
/// let mut writer = LimitWriter::new(Vec::new(), 3);
///
/// assert_eq!(3, writer.write(b"abcdef")?);
/// assert_eq!(0, writer.remaining());
/// assert_eq!(0, writer.write(b"x")?);
/// assert_eq!(b"abc", writer.inner().as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Type Parameters
///
/// - `W`: Wrapped writer type.
#[must_use]
pub struct LimitWriter<W> {
    /// Writer constrained by this wrapper.
    inner: W,
    /// Number of bytes still accepted.
    remaining: u64,
}

impl<W> LimitWriter<W> {
    /// Creates a writer that accepts at most `limit` bytes.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    /// - `limit`: Maximum number of bytes that may be written through this
    ///   wrapper.
    ///
    /// # Returns
    /// A new limited writer.
    #[inline(always)]
    pub fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the number of bytes still accepted by this wrapper.
    ///
    /// # Returns
    /// Remaining writable byte count before the wrapper reports zero writes.
    #[inline(always)]
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline(always)]
    #[must_use]
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// Writes performed directly on the returned writer bypass this wrapper's
    /// remaining-byte limit.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> Write for LimitWriter<W>
where
    W: Write,
{
    /// Writes at most the remaining byte limit.
    ///
    /// The method returns zero without touching `inner` when the limit is
    /// exhausted or `buffer` is empty.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Source byte slice.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes accepted within the remaining limit.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped writer without consuming the
    /// remaining limit.
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        if self.remaining == 0 || buffer.is_empty() {
            return Ok(0);
        }
        let requested = self.remaining.min(buffer.len() as u64) as usize;
        let count = self.inner.write(&buffer[..requested])?;
        self.remaining -= count as u64;
        Ok(count)
    }

    /// Flushes the wrapped writer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the wrapped writer is flushed.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped writer.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
