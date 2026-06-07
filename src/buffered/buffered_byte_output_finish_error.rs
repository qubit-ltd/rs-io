// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    error::Error as StdError,
    fmt,
    io::Error,
};

use super::buffered_byte_output::BufferedByteOutput;

/// Error returned when finishing a [`BufferedByteOutput`] fails.
///
/// The error preserves the buffered output wrapper so callers can inspect,
/// retry, or dismantle it without losing the wrapped writer or pending bytes.
/// Its [`std::error::Error`] implementation does not require the wrapped
/// writer to implement [`fmt::Debug`].
pub struct BufferedByteOutputFinishError<W> {
    error: Error,
    output: BufferedByteOutput<W>,
}

impl<W> BufferedByteOutputFinishError<W> {
    /// Creates an error from an I/O error and the still-owned output wrapper.
    ///
    /// # Parameters
    ///
    /// * `error` - The I/O error returned while finishing the output.
    /// * `output` - The buffered output wrapper after the failed operation.
    ///
    /// # Returns
    ///
    /// A recoverable finish error.
    #[inline(always)]
    pub(super) fn new(error: Error, output: BufferedByteOutput<W>) -> Self {
        Self { error, output }
    }

    /// Returns the I/O error that interrupted finishing.
    ///
    /// # Returns
    ///
    /// A shared reference to the underlying I/O error.
    #[inline(always)]
    #[must_use]
    pub const fn error(&self) -> &Error {
        &self.error
    }

    /// Consumes this error and returns the buffered output wrapper.
    ///
    /// # Returns
    ///
    /// The buffered output wrapper, including the wrapped writer and any
    /// pending bytes not successfully flushed.
    #[inline(always)]
    #[must_use]
    pub fn into_output(self) -> BufferedByteOutput<W> {
        self.output
    }

    /// Consumes this error and returns both the I/O error and output wrapper.
    ///
    /// # Returns
    ///
    /// The I/O error and the recoverable buffered output wrapper.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (Error, BufferedByteOutput<W>) {
        (self.error, self.output)
    }
}

impl<W> fmt::Display for BufferedByteOutputFinishError<W> {
    /// Formats the underlying I/O error.
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<W> fmt::Debug for BufferedByteOutputFinishError<W> {
    /// Formats the recoverable finish error without requiring `W: Debug`.
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BufferedByteOutputFinishError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<W> StdError for BufferedByteOutputFinishError<W> {}
