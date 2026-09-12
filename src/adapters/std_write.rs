// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Adapter from a Qubit synchronous output to the standard library writer.

use std::io::Result;
use std::io::Write;

use crate::Output;

/// Adapts an [`Output`] of bytes to [`std::io::Write`].
#[must_use]
#[repr(transparent)]
pub struct StdWrite<O> {
    /// Wrapped byte output.
    inner: O,
}

impl<O> StdWrite<O> {
    /// Creates a standard writer around `inner`.
    #[inline]
    pub const fn new(inner: O) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped output.
    #[inline]
    pub const fn get_ref(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    #[inline]
    pub fn get_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Unwraps and returns the wrapped output.
    #[inline]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> Write for StdWrite<O>
where
    O: Output<Item = u8>,
{
    #[inline]
    fn write(&mut self, input: &[u8]) -> Result<usize> {
        Output::write(&mut self.inner, input)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Output::flush(&mut self.inner)
    }
}
