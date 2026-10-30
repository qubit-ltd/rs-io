// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use crate::Output;
use crate::Seekable;

/// Adapts a mutable borrowed output where an owned [`Output`] value is
/// required.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the mutable output borrow.
/// - `O`: The borrowed output type, which may be unsized.
///
/// # Examples
///
/// ```
/// use qubit_io::OutputRef;
/// use qubit_io::Output;
/// use std::io::Cursor;
///
/// let mut destination = Cursor::new(Vec::new());
/// let mut output = OutputRef::new(&mut destination);
/// assert_eq!(output.write(b"hello").unwrap(), 5);
/// assert_eq!(output.into_inner().get_ref(), b"hello");
/// ```
#[must_use]
#[repr(transparent)]
pub struct OutputRef<'a, O>
where
    O: Output + ?Sized,
{
    /// Mutable output borrow being adapted.
    inner: &'a mut O,
}

impl<'a, O> OutputRef<'a, O>
where
    O: Output + ?Sized,
{
    /// Creates an output adapter that borrows `inner` mutably.
    ///
    /// # Parameters
    ///
    /// - `inner`: Mutable output borrow to adapt.
    ///
    /// # Returns
    ///
    /// Returns an adapter that retains the borrow for `'a`.
    #[inline(always)]
    pub const fn new(inner: &'a mut O) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the borrowed output.
    ///
    /// # Returns
    ///
    /// Returns the borrowed output with shared access.
    #[inline(always)]
    #[must_use]
    pub fn get_ref(&self) -> &O {
        self.inner
    }

    /// Returns mutable access to the borrowed output.
    ///
    /// # Returns
    ///
    /// Returns the borrowed output with mutable access.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self) -> &mut O {
        self.inner
    }

    /// Consumes this adapter and returns its mutable output borrow.
    ///
    /// # Returns
    ///
    /// Returns the original mutable borrow.
    #[inline(always)]
    #[must_use]
    pub fn into_inner(self) -> &'a mut O {
        self.inner
    }
}

impl<O> Output for OutputRef<'_, O>
where
    O: Output + ?Sized,
{
    /// Item type written to the borrowed output.
    type Item = O::Item;

    /// Returns the wrapped output's buffering capability.
    ///
    /// # Returns
    ///
    /// Returns `true` when the borrowed output is buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Forwards an unchecked write to the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// Returns the number of items written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline(always)]
    unsafe fn write_unchecked(&mut self, input: &[Self::Item], index: usize, count: usize) -> io::Result<usize> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.write_unchecked(input, index, count) }
    }

    /// Forwards a checked write to the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    ///
    /// # Returns
    ///
    /// Returns the number of items written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> io::Result<usize> {
        self.inner.write(input)
    }

    /// Forwards a vectored write to the borrowed output.
    ///
    /// # Parameters
    ///
    /// - `buffers`: Source slices passed to the borrowed output.
    ///
    /// # Returns
    ///
    /// Returns the number of items actually written, which may be less than
    /// the total length of the slices.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output. This adapter does not
    /// guarantee that the underlying output uses scatter/gather I/O.
    #[inline(always)]
    fn write_vectored(&mut self, buffers: &[&[Self::Item]]) -> io::Result<usize> {
        self.inner.write_vectored(buffers)
    }

    /// Forwards an unchecked complete write to the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    /// - `index`: Starting source index.
    /// - `count`: Number of items to write.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after every requested item is written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output, including premature
    /// write-zero failures.
    ///
    /// # Safety
    ///
    /// The range `index..index + count` must be valid for `input`.
    #[inline(always)]
    unsafe fn write_fully_unchecked(&mut self, input: &[Self::Item], index: usize, count: usize) -> io::Result<()> {
        // SAFETY: The caller's valid-range guarantee is forwarded unchanged.
        unsafe { self.inner.write_fully_unchecked(input, index, count) }
    }

    /// Forwards a complete write to the wrapped output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after every item is written.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output, including premature
    /// write-zero failures.
    #[inline(always)]
    fn write_fully(&mut self, input: &[Self::Item]) -> io::Result<()> {
        self.inner.write_fully(input)
    }

    /// Flushes the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the borrowed output is flushed.
    ///
    /// # Errors
    ///
    /// Returns an error reported while flushing the borrowed output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<O> Seekable for OutputRef<'_, O>
where
    O: Output + Seekable + ?Sized,
{
    /// The seek unit inherited from the borrowed output.
    type Unit = O::Unit;

    /// Seeks the borrowed output to a position and returns its new absolute
    /// position.
    ///
    /// # Parameters
    ///
    /// - `position`: Position to seek to, measured in the inner output's units.
    ///
    /// # Returns
    ///
    /// Returns the inner output's new absolute position.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the borrowed output.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        Seekable::seek_to(self.inner, position)
    }
}
