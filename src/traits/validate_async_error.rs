// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Error,
    ErrorKind,
};

/// Converts error kinds forbidden by the asynchronous I/O contract.
///
/// # Parameters
///
/// - `error`: Error returned by an asynchronous implementation.
///
/// # Returns
///
/// Returns an [`ErrorKind::InvalidData`] contract error for `WouldBlock` and
/// `Interrupted`; otherwise, returns `error` unchanged.
#[inline]
pub(crate) fn validate_async_error(error: Error) -> Error {
    match error.kind() {
        ErrorKind::WouldBlock => Error::new(
            ErrorKind::InvalidData,
            "asynchronous I/O implementation returned WouldBlock",
        ),
        ErrorKind::Interrupted => Error::new(
            ErrorKind::InvalidData,
            "asynchronous I/O implementation returned Interrupted",
        ),
        _ => error,
    }
}
