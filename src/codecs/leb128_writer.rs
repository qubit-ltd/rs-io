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

use crate::{
    Leb128WriteExt,
    StringWriteExt,
};

/// Writer wrapper for LEB128 integers and LEB128 length-prefixed strings.
///
/// # Examples
/// ```
/// use qubit_io::Leb128Writer;
///
/// let mut output = Leb128Writer::new(Vec::new());
/// output.write_uleb_u16(300)?;
///
/// assert_eq!(vec![0xac, 0x02], output.into_inner());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct Leb128Writer<W> {
    inner: W,
}

impl<W> Leb128Writer<W> {
    /// Creates a LEB128 writer.
    ///
    /// # Parameters
    /// - `inner`: Writer to wrap.
    ///
    /// # Returns
    /// A new LEB128 writer.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Returns an immutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer reference.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consumes this wrapper and returns the wrapped writer.
    ///
    /// # Returns
    /// The wrapped writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

macro_rules! delegate_write {
    ($name:ident, $value:ty) => {
        #[doc = concat!("Writes a LEB128 `", stringify!($value), "`.")]
        ///
        /// # Parameters
        /// - `value`: Value to encode.
        ///
        /// # Errors
        /// Returns an I/O error from the wrapped writer.
        pub fn $name(&mut self, value: $value) -> Result<()> {
            self.inner.$name(value)
        }
    };
}

impl<W> Leb128Writer<W>
where
    W: Write,
{
    delegate_write!(write_uleb_u8, u8);
    delegate_write!(write_uleb_u16, u16);
    delegate_write!(write_uleb_u32, u32);
    delegate_write!(write_uleb_u64, u64);
    delegate_write!(write_uleb_u128, u128);
    delegate_write!(write_uleb_usize, usize);
    delegate_write!(write_sleb_i8, i8);
    delegate_write!(write_sleb_i16, i16);
    delegate_write!(write_sleb_i32, i32);
    delegate_write!(write_sleb_i64, i64);
    delegate_write!(write_sleb_i128, i128);
    delegate_write!(write_sleb_isize, isize);

    /// Writes a UTF-8 string with an unsigned LEB128 byte-length prefix.
    ///
    /// # Parameters
    /// - `value`: String slice to write.
    ///
    /// # Errors
    /// Returns an I/O error from the wrapped writer.
    pub fn write_utf8_string(&mut self, value: &str) -> Result<()> {
        self.inner.write_utf8_string_uleb(value)
    }
}

impl<W> Write for Leb128Writer<W>
where
    W: Write,
{
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
