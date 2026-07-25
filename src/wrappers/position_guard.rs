// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
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
/// must be observed. A failed drop-time restore does not panic and is not
/// logged by this type.
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
///     guard.inner_mut().seek(SeekFrom::End(0))?;
/// }
///
/// assert_eq!(2, stream.position());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the guarded stream borrow.
/// - `S`: Seekable stream type.
#[must_use = "dropping an unused guard restores the captured position immediately"]
pub struct PositionGuard<'a, S>
where
    S: Seek + ?Sized,
{
    /// Mutable stream borrow retained by the guard.
    stream: &'a mut S,
    /// Position captured when the guard was created.
    position: u64,
    /// Whether automatic restoration has been disabled.
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
    #[inline]
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
    #[inline(always)]
    #[must_use]
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Returns a mutable reference to the guarded stream.
    ///
    /// # Returns
    /// The guarded stream reference.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut S {
        self.stream
    }

    /// Restores the captured position immediately.
    ///
    /// After a successful restore, drop-time restoration is disabled. If
    /// restoring fails, drop will still make a best-effort restore attempt.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after restoring the captured position.
    ///
    /// # Errors
    /// Returns the error reported by [`Seek::seek`] when the stream cannot seek
    /// back to the captured position.
    #[inline(always)]
    pub fn restore(&mut self) -> Result<()> {
        self.stream.seek(SeekFrom::Start(self.position)).map(|_| {
            self.done = true;
        })
    }

    /// Disables drop-time restoration without moving the stream.
    ///
    /// This is useful when the caller intentionally wants to keep the stream at
    /// its current position.
    #[inline(always)]
    pub fn dismiss(mut self) {
        self.done = true;
    }
}

impl<S> Drop for PositionGuard<'_, S>
where
    S: Seek + ?Sized,
{
    /// Attempts best-effort restoration when it remains enabled.
    ///
    /// Any seek error is ignored because destructors cannot report it.
    fn drop(&mut self) {
        if !self.done {
            drop(self.stream.seek(SeekFrom::Start(self.position)));
        }
    }
}
