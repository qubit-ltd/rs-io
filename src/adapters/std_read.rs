// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Adapter from a Qubit synchronous input to the standard library reader.

use std::io::Read;
use std::io::Result;

use crate::Input;

/// Adapts an [`Input`] of bytes to [`std::io::Read`].
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
