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
    Result,
    Seek,
    SeekFrom,
};

/// Guard that restores a seekable stream to its original position.
///
/// `PositionGuard` captures the stream position when it is created and seeks
/// back to that position when the guard is dropped, unless
/// [`PositionGuard::restore`] or [`PositionGuard::dismiss`] has already
/// completed. Drop-time restoration errors are ignored because [`Drop::drop`]
/// cannot return a [`Result`]; call [`PositionGuard::restore`] when the error
/// must be observed.
///
/// # Examples
/// ```
/// use std::io::{
///     Cursor,
///     Seek,
///     SeekFrom,
/// };
///
/// use qubit_io::PositionGuard;
///
/// let mut stream = Cursor::new(b"abcdef".to_vec());
/// stream.seek(SeekFrom::Start(2))?;
///
/// {
///     let mut guard = PositionGuard::new(&mut stream)?;
///     guard.get_mut().seek(SeekFrom::End(0))?;
/// }
///
/// assert_eq!(2, stream.position());
/// # Ok::<(), std::io::Error>(())
/// ```
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
