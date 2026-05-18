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
