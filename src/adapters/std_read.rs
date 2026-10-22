// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Adapter from a Qubit synchronous input to the standard library reader.

use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

use crate::Input;

/// Adapts an [`Input`] of bytes to [`std::io::Read`].
///
/// # Type Parameters
///
/// - `I`: Input that supplies bytes to the standard reader.
///
/// # Examples
///
/// ```
/// use qubit_io::StdRead;
/// use std::io::Read;
///
/// let input = std::io::Cursor::new(b"hello".to_vec());
/// let mut reader = StdRead::new(input);
/// let mut bytes = Vec::new();
/// reader.read_to_end(&mut bytes).expect("reading should succeed");
/// assert_eq!(bytes, b"hello");
/// ```
#[must_use]
#[repr(transparent)]
pub struct StdRead<I> {
    /// Wrapped byte input.
    inner: I,
}

impl<I> StdRead<I> {
    /// Creates a standard reader around `inner`.
    #[inline]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped input.
    #[inline]
    pub const fn get_ref(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    #[inline]
    pub fn get_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Unwraps and returns the wrapped input.
    #[inline]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> Read for StdRead<I>
where
    I: Input<Item = u8>,
{
    #[inline]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        Input::read(&mut self.inner, output)
    }
}

impl<I> Seek for StdRead<I>
where
    I: Input<Item = u8> + crate::Seekable<Unit = u8>,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        crate::Seekable::seek_to(&mut self.inner, position)
    }
}
