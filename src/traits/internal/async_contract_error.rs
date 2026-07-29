// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use core::fmt;
use std::error::Error as StdError;
use std::io::Error;

/// Describes an asynchronous I/O contract violation and its original error.
#[must_use]
#[derive(Debug)]
pub(super) struct AsyncContractError {
    /// Explanation of the violated asynchronous I/O contract.
    message: &'static str,
    /// Original error returned by the asynchronous I/O implementation.
    source: Error,
}

impl AsyncContractError {
    /// Creates an error that retains the contract explanation and source error.
    ///
    /// # Parameters
    ///
    /// - `message`: Static explanation of the violated contract.
    /// - `source`: Original I/O error returned by the implementation.
    ///
    /// # Returns
    ///
    /// Returns an error containing both pieces of context.
    #[inline(always)]
    pub(super) const fn new(message: &'static str, source: Error) -> Self {
        Self { message, source }
    }
}

impl fmt::Display for AsyncContractError {
    /// Formats the contract violation together with its original error.
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
    /// Returns a formatting error if the destination rejects the output.
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
    /// Always returns `Some` containing the original I/O error.
    #[inline(always)]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}
