// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::io;
use std::sync::Arc;

/// Keeps a latched tee error available to repeated reports.
#[derive(Debug)]
pub(super) struct SharedTeeError {
    /// Original branch error shared with the tee failure state.
    original: Arc<io::Error>,
}

impl SharedTeeError {
    /// Shares the original branch error without changing its diagnostics.
    #[inline]
    pub(super) fn new(original: Arc<io::Error>) -> Self {
        Self { original }
    }
}

impl Display for SharedTeeError {
    /// Formats the original branch error without adding a second message.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.original.as_ref(), formatter)
    }
}

impl Error for SharedTeeError {
    /// Exposes the original I/O error as the next diagnostic node.
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.original.as_ref())
    }
}
