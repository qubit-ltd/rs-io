// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use core::fmt;
use std::io;

/// Error returned when a consuming buffered-I/O conversion cannot finish.
///
/// The value retains both the I/O error and the original buffered object so
/// callers can inspect pending state, repair a transient failure, and retry.
///
/// # Type Parameters
///
/// - `T`: Buffered object retained after the failed conversion.
#[derive(Debug)]
pub struct IntoInnerError<T> {
    error: io::Error,
    inner: T,
}

impl<T> IntoInnerError<T> {
    /// Creates a recoverable consuming-conversion error.
    ///
    /// # Parameters
    ///
    /// - `error`: I/O error that prevented the conversion.
    /// - `inner`: Buffered object retained after the failure.
    ///
    /// # Returns
    ///
    /// Returns an error containing both supplied values.
    #[must_use]
    pub const fn new(error: io::Error, inner: T) -> Self {
        Self { error, inner }
    }

    /// Returns the I/O error that prevented conversion.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained I/O error.
    #[must_use]
    #[inline(always)]
    pub const fn error(&self) -> &io::Error {
        &self.error
    }

    /// Returns the buffered object retained after conversion failed.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained buffered object.
    #[must_use]
    #[inline(always)]
    pub const fn inner(&self) -> &T {
        &self.inner
    }

    /// Returns the buffered object retained after conversion failed.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained buffered object.
    #[must_use]
    #[inline(always)]
    pub const fn writer(&self) -> &T {
        self.inner()
    }

    /// Returns mutable access to the buffered object retained after failure.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the retained buffered object.
    #[inline(always)]
    pub const fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Returns mutable access to the buffered object retained after failure.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the retained buffered object.
    #[inline(always)]
    pub const fn writer_mut(&mut self) -> &mut T {
        self.inner_mut()
    }

    /// Consumes this error and returns the buffered object.
    ///
    /// # Returns
    ///
    /// Returns the retained buffered object and discards the I/O error.
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consumes this error and returns the buffered object.
    ///
    /// # Returns
    ///
    /// Returns the retained buffered object and discards the I/O error.
    #[must_use]
    #[inline(always)]
    pub fn into_writer(self) -> T {
        self.into_inner()
    }

    /// Consumes this error and returns the underlying I/O error.
    ///
    /// # Returns
    ///
    /// Returns the retained I/O error and discards the buffered object.
    #[must_use]
    #[inline(always)]
    pub fn into_error(self) -> io::Error {
        self.error
    }

    /// Consumes this error and returns both retained values.
    ///
    /// # Returns
    ///
    /// Returns `(error, buffered_object)`.
    #[must_use]
    #[inline(always)]
    pub fn into_parts(self) -> (io::Error, T) {
        (self.error, self.inner)
    }
}

impl<T> fmt::Display for IntoInnerError<T> {
    /// Formats the retained I/O error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T> std::error::Error for IntoInnerError<T>
where
    T: fmt::Debug,
{
    /// Returns the retained I/O error as the source.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
