// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::collections::TryReserveError;
use std::io::{
    Error,
    Result,
};

/// Converts a fallible allocation error into an I/O error.
///
/// # Parameters
///
/// - `error`: Allocation failure reported by a collection.
///
/// # Returns
///
/// Returns an [`ErrorKind::Other`] error carrying allocation context.
fn allocation_error(error: TryReserveError) -> Error {
    Error::other(format!("failed to reserve output buffer capacity: {error}"))
}

/// Reserves capacity in a vector and reports allocation failure as an I/O
/// error.
///
/// # Parameters
///
/// - `output`: Vector that will receive additional elements.
/// - `additional`: Number of additional elements to reserve.
///
/// # Errors
///
/// Returns [`ErrorKind::Other`] if the allocation request fails.
pub(crate) fn try_reserve_vec<T>(
    output: &mut Vec<T>,
    additional: usize,
) -> Result<()> {
    output.try_reserve(additional).map_err(allocation_error)
}

/// Reserves capacity in a string and reports allocation failure as an I/O
/// error.
///
/// # Parameters
///
/// - `output`: String that will receive additional bytes.
/// - `additional`: Number of additional bytes to reserve.
///
/// # Errors
///
/// Returns [`ErrorKind::Other`] if the allocation request fails.
pub(crate) fn try_reserve_string(
    output: &mut String,
    additional: usize,
) -> Result<()> {
    output.try_reserve(additional).map_err(allocation_error)
}
