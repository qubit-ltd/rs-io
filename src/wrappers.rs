/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
// qubit-style: allow multiple-public-types
use std::io::{
    Read,
    Result,
    Seek,
    SeekFrom,
    Write,
};

/// Reader wrapper that exposes at most a fixed number of bytes.
///
/// `LimitReader` is useful when a parser must consume a bounded section of a
/// larger stream without relying on the caller to provide a pre-sliced buffer.
/// Once the remaining limit reaches zero, reads return `Ok(0)` without touching
/// the inner reader.
pub struct LimitReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> LimitReader<R> {
    /// Creates a reader that exposes at most `limit` bytes from `inner`.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `limit`: Maximum number of bytes that may be read through this wrapper.
    ///
    /// # Returns
    /// A new limited reader.
    pub fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }

    /// Returns the number of bytes still available through this wrapper.
    ///
    /// # Returns
    /// Remaining readable byte count before the wrapper reports EOF.
    pub fn remaining(&self) -> u64 {
        self.remaining
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

impl<R> Read for LimitReader<R>
where
    R: Read,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        if self.remaining == 0 || buffer.is_empty() {
            return Ok(0);
        }
        let requested = self.remaining.min(buffer.len() as u64) as usize;
        let count = self.inner.read(&mut buffer[..requested])?;
        self.remaining -= count as u64;
        Ok(count)
    }
}

/// Writer wrapper that accepts at most a fixed number of bytes.
///
/// `LimitWriter` mirrors [`LimitReader`] for output streams. Once the remaining
/// limit reaches zero, [`Write::write`] returns `Ok(0)` without touching the
/// inner writer. Callers using [`Write::write_all`] will therefore receive the
/// standard write-zero error when trying to write beyond the limit.
pub struct LimitWriter<W> {
    inner: W,
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
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> Write for LimitWriter<W>
where
    W: Write,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        if self.remaining == 0 || buffer.is_empty() {
            return Ok(0);
        }
        let requested = self.remaining.min(buffer.len() as u64) as usize;
        let count = self.inner.write(&buffer[..requested])?;
        self.remaining -= count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

/// Reader wrapper that counts successfully read bytes.
///
/// The count is increased only after the wrapped reader reports a successful
/// byte count. Failed reads do not change the counter.
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

/// Writer wrapper that counts successfully written bytes.
///
/// The count is increased only after the wrapped writer accepts bytes. Failed
/// writes do not change the counter.
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
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
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

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

/// Reader wrapper that copies bytes read from a source into a branch writer.
///
/// `TeeReader` is useful for digesting, logging, caching, or mirroring a stream
/// while it is consumed. If the branch writer fails, the source bytes have
/// already been read from the inner reader and the branch error is returned.
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
    pub fn new(reader: R, branch: W) -> Self {
        Self { reader, branch }
    }

    /// Returns an immutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    pub fn reader_ref(&self) -> &R {
        &self.reader
    }

    /// Returns a mutable reference to the source reader.
    ///
    /// # Returns
    /// The source reader reference.
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    pub fn branch_ref(&self) -> &W {
        &self.branch
    }

    /// Returns a mutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    pub fn branch_mut(&mut self) -> &mut W {
        &mut self.branch
    }

    /// Consumes this wrapper and returns the source reader and branch writer.
    ///
    /// # Returns
    /// A tuple containing the source reader and branch writer.
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

/// Writer wrapper that mirrors accepted bytes into a branch writer.
///
/// `TeeWriter` writes to the primary writer first. The exact bytes accepted by
/// the primary writer are then written to the branch writer with
/// [`Write::write_all`]. If the branch writer fails, the primary writer may
/// already have accepted bytes and the branch error is returned.
pub struct TeeWriter<P, B> {
    primary: P,
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
    pub fn new(primary: P, branch: B) -> Self {
        Self { primary, branch }
    }

    /// Returns an immutable reference to the primary writer.
    ///
    /// # Returns
    /// The primary writer reference.
    pub fn primary_ref(&self) -> &P {
        &self.primary
    }

    /// Returns a mutable reference to the primary writer.
    ///
    /// # Returns
    /// The primary writer reference.
    pub fn primary_mut(&mut self) -> &mut P {
        &mut self.primary
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    pub fn branch_ref(&self) -> &B {
        &self.branch
    }

    /// Returns a mutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns both wrapped writers.
    ///
    /// # Returns
    /// A tuple containing the primary writer and branch writer.
    pub fn into_inner(self) -> (P, B) {
        (self.primary, self.branch)
    }
}

impl<P, B> Write for TeeWriter<P, B>
where
    P: Write,
    B: Write,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        let count = self.primary.write(buffer)?;
        self.branch.write_all(&buffer[..count])?;
        Ok(count)
    }

    fn flush(&mut self) -> Result<()> {
        self.primary.flush()?;
        self.branch.flush()
    }
}

/// Guard that restores a seekable stream to its captured position.
///
/// The guard captures the stream position on construction. It restores that
/// position on drop unless [`PositionGuard::restore`] or
/// [`PositionGuard::dismiss`] has already completed. Drop-time restoration
/// errors are ignored because [`Drop::drop`] cannot return a [`Result`]; call
/// [`PositionGuard::restore`] when the error must be observed.
pub struct PositionGuard<'a, S>
where
    S: Seek + ?Sized,
{
    stream: &'a mut S,
    position: u64,
    done: bool,
}

impl<'a, S> PositionGuard<'a, S>
where
    S: Seek + ?Sized,
{
    /// Captures the current position of `stream`.
    ///
    /// # Parameters
    /// - `stream`: Seekable stream to guard.
    ///
    /// # Returns
    /// A guard that will restore the captured position on drop.
    ///
    /// # Errors
    /// Returns the error reported by [`Seek::stream_position`] when the current
    /// position cannot be read.
    pub fn new(stream: &'a mut S) -> Result<Self> {
        let position = stream.stream_position()?;
        Ok(Self {
            stream,
            position,
            done: false,
        })
    }

    /// Returns the captured stream position.
    ///
    /// # Returns
    /// The position captured when this guard was created.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Returns a mutable reference to the guarded stream.
    ///
    /// # Returns
    /// The guarded stream reference.
    pub fn get_mut(&mut self) -> &mut S {
        self.stream
    }

    /// Restores the captured position immediately.
    ///
    /// After a successful restore, drop-time restoration is disabled. If
    /// restoring fails, drop will still make a best-effort restore attempt.
    ///
    /// # Errors
    /// Returns the error reported by [`Seek::seek`] when the stream cannot seek
    /// back to the captured position.
    pub fn restore(&mut self) -> Result<()> {
        self.stream.seek(SeekFrom::Start(self.position)).map(|_| {
            self.done = true;
        })
    }

    /// Disables drop-time restoration without moving the stream.
    ///
    /// This is useful when the caller intentionally wants to keep the stream at
    /// its current position.
    pub fn dismiss(mut self) {
        self.done = true;
    }
}

impl<S> Drop for PositionGuard<'_, S>
where
    S: Seek + ?Sized,
{
    fn drop(&mut self) {
        if !self.done {
            drop(self.stream.seek(SeekFrom::Start(self.position)));
        }
    }
}
