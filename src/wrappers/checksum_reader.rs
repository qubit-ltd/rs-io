/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::hash::Hasher;
use std::io::{
    Read,
    Result,
};

/// implements [`Hasher`]. Failed reads do not update the hasher.
pub struct ChecksumReader<R, H> {
    inner: R,
    hasher: H,
}

impl<R, H> ChecksumReader<R, H>
where
    H: Hasher,
{
    /// Creates a checksum reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    /// - `hasher`: Hasher updated with successfully read bytes.
    ///
    /// # Returns
    /// A new checksum reader.
    pub fn new(inner: R, hasher: H) -> Self {
        Self { inner, hasher }
    }

    /// Returns the current checksum value.
    ///
    /// # Returns
    /// The value reported by [`Hasher::finish`].
    pub fn checksum(&self) -> u64 {
        self.hasher.finish()
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

    /// Returns an immutable reference to the wrapped hasher.
    ///
    /// # Returns
    /// The wrapped hasher reference.
    pub fn hasher_ref(&self) -> &H {
        &self.hasher
    }

    /// Returns a mutable reference to the wrapped hasher.
    ///
    /// # Returns
    /// The wrapped hasher reference.
    pub fn hasher_mut(&mut self) -> &mut H {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns the wrapped reader and hasher.
    ///
    /// # Returns
    /// A tuple containing the wrapped reader and hasher.
    pub fn into_inner(self) -> (R, H) {
        (self.inner, self.hasher)
    }
}

impl<R, H> Read for ChecksumReader<R, H>
where
    R: Read,
    H: Hasher,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.inner.read(buffer)?;
        self.hasher.write(&buffer[..count]);
        Ok(count)
    }
}
