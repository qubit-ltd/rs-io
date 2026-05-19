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

use crate::ZigZagWriteExt;

/// Writer wrapper for ZigZag encoded signed integers.
///
/// # Examples
/// ```
/// use qubit_io::ZigZagWriter;
///
/// let mut output = ZigZagWriter::new(Vec::new());
/// output.write_i32(-1)?;
///
/// assert_eq!(vec![0x01], output.into_inner());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct ZigZagWriter<W> {
    inner: W,
}

impl<W> ZigZagWriter<W> {
    /// Creates a ZigZag writer.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    ///
    /// # Returns
    /// A new ZigZag writer.
    #[inline]
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

macro_rules! delegate_write {
    ($name:ident, $inner:ident, $value:ty) => {
        #[doc = concat!("Writes a ZigZag encoded `", stringify!($value), "`.")]
        ///
        /// # Parameters
        /// - `value`: Value to encode.
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped writer.
        #[inline]
        pub fn $name(&mut self, value: $value) -> Result<()> {
            self.inner.$inner(value)
        }
    };
}

impl<W> ZigZagWriter<W>
where
    W: Write,
{
    delegate_write!(write_i8, write_zigzag_i8, i8);
    delegate_write!(write_i16, write_zigzag_i16, i16);
    delegate_write!(write_i32, write_zigzag_i32, i32);
    delegate_write!(write_i64, write_zigzag_i64, i64);
    delegate_write!(write_i128, write_zigzag_i128, i128);
    delegate_write!(write_isize, write_zigzag_isize, isize);
}

impl<W> Write for ZigZagWriter<W>
where
    W: Write,
{
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.inner.write(buffer)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
