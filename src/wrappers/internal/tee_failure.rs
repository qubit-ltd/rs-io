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
    error: Option<Arc<io::Error>>,
}

impl TeeFailure {
    pub(crate) const fn new() -> Self {
        Self { error: None }
    }

    pub(crate) fn error(&self) -> Option<&io::Error> {
        self.error.as_deref()
    }

    pub(crate) fn record(&mut self, error: io::Error) {
        if self.error.is_none() {
            self.error = Some(Arc::new(error));
        }
    }

    pub(crate) fn check(&self) -> io::Result<()> {
        match &self.error {
            Some(error) => Err(Self::shared_error(Arc::clone(error))),
            None => Ok(()),
        }
    }

    pub(crate) fn into_error(self) -> Option<io::Error> {
        self.error
            .map(|error| Arc::try_unwrap(error).unwrap_or_else(Self::shared_error))
    }

    fn shared_error(error: Arc<io::Error>) -> io::Error {
        let kind = if error.kind() == io::ErrorKind::Interrupted {
            io::ErrorKind::Other
        } else {
            error.kind()
        };
        io::Error::new(kind, error)
    }
}
