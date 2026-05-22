/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use core::fmt;
use std::error::Error;

use crate::Leb128DecodeErrorKind;

/// Error reported while decoding a LEB128 integer from a byte buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Leb128DecodeError {
    kind: Leb128DecodeErrorKind,
    index: usize,
}

impl Leb128DecodeError {
    /// Creates a LEB128 decoding error.
    ///
    /// # Parameters
    ///
    /// - `kind`: Failure category.
    /// - `index`: Absolute byte index at which the failure was detected.
    ///
    /// # Returns
    ///
    /// Returns a decoding error carrying the supplied context.
    #[inline]
    pub const fn new(kind: Leb128DecodeErrorKind, index: usize) -> Self {
        Self { kind, index }
    }

    /// Returns the decoding error kind.
    #[must_use]
    #[inline]
    pub const fn kind(self) -> Leb128DecodeErrorKind {
        self.kind
    }

    /// Returns the absolute byte index associated with this error.
    #[must_use]
    #[inline]
    pub const fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for Leb128DecodeError {
    /// Formats the LEB128 decoding error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.kind)
    }
}

impl Error for Leb128DecodeError {}
