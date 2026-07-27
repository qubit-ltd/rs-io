// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use core::fmt;
use std::error::Error as StdError;
use std::io;

/// Describes an asynchronous I/O contract violation and its original error.
#[must_use]
#[derive(Debug)]
pub(in crate::traits) struct AsyncContractError {
    /// Explanation of the violated asynchronous I/O contract.
    message: &'static str,
    /// Original error returned by the asynchronous I/O implementation.
    source: io::Error,
}

impl AsyncContractError {
    /// Creates an asynchronous I/O contract error.
    ///
    /// # Parameters
    ///
    /// - `message`: Explanation of the contract violation.
    /// - `source`: Original error returned by the implementation.
    ///
    /// # Returns
    ///
    /// Returns an error retaining both supplied values.
    #[inline(always)]
    pub(in crate::traits) const fn new(message: &'static str, source: io::Error) -> Self {
        Self { message, source }
    }
}

impl fmt::Display for AsyncContractError {
    /// Formats the contract violation and original error.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    ///
    /// Returns the formatter result.
    ///
    /// # Errors
    ///
    /// Returns an error if the formatter cannot accept the output.
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.message, self.source)
    }
}

impl StdError for AsyncContractError {
    /// Returns the original I/O error as the source.
    ///
    /// # Returns
    ///
    /// Returns the original error as a trait object.
    #[inline(always)]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}
