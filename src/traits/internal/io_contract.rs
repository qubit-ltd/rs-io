// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;

use super::async_contract_error::AsyncContractError;

/// Normalizes error kinds forbidden by the asynchronous I/O contract.
///
/// # Parameters
///
/// - `error`: Error reported by an asynchronous I/O implementation.
///
/// # Returns
///
/// Returns an [`ErrorKind::InvalidData`] error retaining the original error for
/// [`ErrorKind::WouldBlock`] and [`ErrorKind::Interrupted`]. Returns every
/// other error unchanged.
#[must_use]
pub(crate) fn normalize_async_error(error: Error) -> Error {
    match error.kind() {
        ErrorKind::WouldBlock => Error::new(
            ErrorKind::InvalidData,
            AsyncContractError::new("asynchronous I/O implementation returned WouldBlock", error),
        ),
        ErrorKind::Interrupted => Error::new(
            ErrorKind::InvalidData,
            AsyncContractError::new("asynchronous I/O implementation returned Interrupted", error),
        ),
        _ => error,
    }
}

/// Validates a count reported by an input implementation.
///
/// # Parameters
///
/// - `read`: Item count reported by the implementation.
/// - `requested`: Maximum item count requested by the caller.
///
/// # Returns
///
/// Returns `Ok(())` when `read` does not exceed `requested`.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when `read` exceeds `requested`.
#[inline]
pub(crate) fn validate_read_count(read: usize, requested: usize) -> Result<()> {
    if read > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("reader reported {read} items for a {requested}-item buffer"),
        ));
    }
    Ok(())
}

/// Validates a count reported by an output implementation.
///
/// # Parameters
///
/// - `written`: Item count reported by the implementation.
/// - `requested`: Maximum item count requested by the caller.
///
/// # Returns
///
/// Returns `Ok(())` when `written` does not exceed `requested`.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when `written` exceeds `requested`.
#[inline]
pub(crate) fn validate_write_count(written: usize, requested: usize) -> Result<()> {
    if written > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("writer reported {written} items for a {requested}-item buffer"),
        ));
    }
    Ok(())
}
