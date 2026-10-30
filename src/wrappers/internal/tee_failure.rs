// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::sync::Arc;

/// Stores the first branch failure shared by tee wrapper operations.
#[derive(Debug)]
pub(crate) struct TeeFailure {
    /// The first branch error, retained behind `Arc` so it can be reported
    /// repeatedly without discarding the original error or its source chain.
    error: Option<Arc<io::Error>>,
}

impl TeeFailure {
    /// Creates a failure state with no recorded branch error.
    pub(crate) const fn new() -> Self {
        Self { error: None }
    }

    /// Borrows the original recorded branch error, if any.
    pub(crate) fn error(&self) -> Option<&io::Error> {
        self.error.as_deref()
    }

    /// Records `error` only if no branch error has been recorded yet.
    ///
    /// Later failures do not replace or clear the first error. Recording a
    /// branch failure also does not roll back progress already made on the
    /// primary path; recovery or compensation is the caller's responsibility.
    pub(crate) fn record(&mut self, error: io::Error) {
        if self.error.is_none() {
            self.error = Some(Arc::new(error));
        }
    }

    /// Returns an error representing the recorded branch failure, if present.
    ///
    /// The shared error retains the original error in its source chain. An
    /// original `Interrupted` error is presented as `Other` so aggregate
    /// operations do not retry a branch failure that has already been latched.
    /// This does not roll back progress already made on the primary path.
    pub(crate) fn check(&self) -> io::Result<()> {
        match &self.error {
            Some(error) => Err(Self::shared_error(Arc::clone(error))),
            None => Ok(()),
        }
    }

    /// Returns the recorded error, if any, recovering the original when unique.
    ///
    /// If the `Arc` is uniquely owned, this returns the original error and its
    /// original kind. Otherwise, it returns a shared representation that keeps
    /// the original in its source chain; an `Interrupted` kind is normalized to
    /// `Other` to prevent retrying the already-latched failure. This does not
    /// roll back progress already made on the primary path.
    pub(crate) fn into_error(self) -> Option<io::Error> {
        self.error
            .map(|error| Arc::try_unwrap(error).unwrap_or_else(Self::shared_error))
    }

    /// Builds a reportable error that retains the original error as its cause.
    ///
    /// `Interrupted` is normalized to `Other` so an aggregate operation does
    /// not retry a branch failure that has already been latched. The primary
    /// path's progress is not rolled back.
    fn shared_error(error: Arc<io::Error>) -> io::Error {
        let kind = if error.kind() == io::ErrorKind::Interrupted {
            io::ErrorKind::Other
        } else {
            error.kind()
        };
        io::Error::new(kind, error)
    }
}
