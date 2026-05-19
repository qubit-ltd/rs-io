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
    Write,
};

/// Writer wrapper that mirrors accepted bytes into a branch writer.
///
/// `TeeWriter` writes to the primary writer first, then writes exactly the
/// accepted bytes into the branch writer with [`Write::write_all`]. If the
/// branch writer fails, the primary writer may already have accepted bytes and
/// the branch error is returned.
///
/// # Examples
/// ```
/// use std::io::Write;
///
/// use qubit_io::TeeWriter;
///
/// let primary = Vec::new();
/// let branch = Vec::new();
/// let mut writer = TeeWriter::new(primary, branch);
///
/// writer.write_all(b"abc")?;
/// let (primary, branch) = writer.into_inner();
///
/// assert_eq!(b"abc", primary.as_slice());
/// assert_eq!(b"abc", branch.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
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
    #[inline]
    pub fn new(primary: P, branch: B) -> Self {
        Self { primary, branch }
    }

    /// Returns an immutable reference to the primary writer.
    ///
    /// # Returns
    /// The primary writer reference.
    #[inline]
    pub fn primary_ref(&self) -> &P {
        &self.primary
    }

    /// Returns a mutable reference to the primary writer.
    ///
    /// # Returns
    /// The primary writer reference.
    #[inline]
    pub fn primary_mut(&mut self) -> &mut P {
        &mut self.primary
    }

    /// Returns an immutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch_ref(&self) -> &B {
        &self.branch
    }

    /// Returns a mutable reference to the branch writer.
    ///
    /// # Returns
    /// The branch writer reference.
    #[inline]
    pub fn branch_mut(&mut self) -> &mut B {
        &mut self.branch
    }

    /// Consumes this wrapper and returns both wrapped writers.
    ///
    /// # Returns
    /// A tuple containing the primary writer and branch writer.
    #[inline]
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
