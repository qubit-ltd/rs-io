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

use crate::ZigZagReadExt;

/// Reader wrapper for ZigZag encoded signed integers.
///
/// # Examples
/// ```
/// use std::io::Cursor;
///
/// use qubit_io::{
///     ZigZagReader,
///     ZigZagWriter,
/// };
///
/// let mut output = ZigZagWriter::new(Vec::new());
/// output.write_zigzag_i32(-123)?;
///
/// let mut input = ZigZagReader::new(Cursor::new(output.into_inner()));
/// assert_eq!(-123, input.read_zigzag_i32()?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct ZigZagReader<R> {
    inner: R,
}

impl<R> ZigZagReader<R> {
    /// Creates a ZigZag reader.
    ///
    /// # Parameters
    /// - `inner`: Reader to wrap.
    ///
    /// # Returns
    /// A new ZigZag reader.
    pub fn new(inner: R) -> Self {
        Self { inner }
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

macro_rules! delegate_read {
    ($name:ident, $value:ty) => {
        #[doc = concat!("Reads a ZigZag encoded `", stringify!($value), "`.")]
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped reader, or `InvalidData` for
        /// malformed or overflowing underlying unsigned LEB128 input.
        pub fn $name(&mut self) -> Result<$value> {
            self.inner.$name()
        }
    };
}

impl<R> ZigZagReader<R>
where
    R: Read,
{
    delegate_read!(read_zigzag_i8, i8);
    delegate_read!(read_zigzag_i8_strict, i8);
    delegate_read!(read_zigzag_i16, i16);
    delegate_read!(read_zigzag_i16_strict, i16);
    delegate_read!(read_zigzag_i32, i32);
    delegate_read!(read_zigzag_i32_strict, i32);
    delegate_read!(read_zigzag_i64, i64);
    delegate_read!(read_zigzag_i64_strict, i64);
    delegate_read!(read_zigzag_i128, i128);
    delegate_read!(read_zigzag_i128_strict, i128);
    delegate_read!(read_zigzag_isize, isize);
    delegate_read!(read_zigzag_isize_strict, isize);
}

impl<R> Read for ZigZagReader<R>
where
    R: Read,
{
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }
}
