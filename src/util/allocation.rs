// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg
#[cfg(coverage)]
use std::cell::Cell;
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
/// Returns an [`std::io::ErrorKind::Other`] error carrying allocation context.
fn allocation_error(error: TryReserveError) -> Error {
    Error::other(format!("failed to reserve output buffer capacity: {error}"))
}

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_FAIL_NEXT_STRING_RESERVE: Cell<bool> = const { Cell::new(false) };
}

/// Makes the next [`try_reserve_vec`] call fail.
///
/// Coverage-only helper for exercising allocation error propagation paths that
/// are impractical to trigger with ordinary test inputs.
#[cfg(coverage)]
pub fn coverage_fail_next_reserve() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(0));
}

/// Makes a later reserve call fail after the given number of successful tries.
///
/// A value of `0` fails on the next reserve call. A value of `1` lets one
/// reserve succeed and fails on the following call.
#[cfg(coverage)]
pub fn coverage_fail_reserve_after(successful_attempts: usize) {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(successful_attempts));
}

/// Makes the next [`try_reserve_string`] call fail without affecting vector
/// reserve calls.
#[cfg(coverage)]
pub fn coverage_fail_next_string_reserve() {
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(true));
}

/// Clears coverage-only reserve hooks between tests.
#[cfg(coverage)]
pub fn coverage_reset_reserve_hooks() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(false));
}

#[cfg(coverage)]
fn coverage_maybe_fail_reserve<T>() -> Option<Result<T>> {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| {
        let remaining = state.get();
        if remaining == usize::MAX {
            return None;
        }
        if remaining == 0 {
            state.set(usize::MAX);
            return Some(Err(Error::other(
                "failed to reserve output buffer capacity: coverage reserve failure",
            )));
        }
        state.set(remaining - 1);
        None
    })
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
/// Returns [`std::io::ErrorKind::Other`] if the allocation request fails.
pub fn try_reserve_vec<T>(
    output: &mut Vec<T>,
    additional: usize,
) -> Result<()> {
    #[cfg(coverage)]
    if let Some(result) = coverage_maybe_fail_reserve::<()>() {
        return result;
    }
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
/// Returns [`std::io::ErrorKind::Other`] if the allocation request fails.
pub fn try_reserve_string(
    output: &mut String,
    additional: usize,
) -> Result<()> {
    #[cfg(coverage)]
    if COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| {
        let fail = state.get();
        if fail {
            state.set(false);
        }
        fail
    }) {
        return Err(Error::other(
            "failed to reserve output buffer capacity: coverage reserve failure",
        ));
    }
    #[cfg(coverage)]
    if let Some(result) = coverage_maybe_fail_reserve::<()>() {
        return result;
    }
    output.try_reserve(additional).map_err(allocation_error)
}

/// Creates a vector with the requested length and initial value.
///
/// # Parameters
///
/// - `len`: Target length of the returned vector.
/// - `fill`: Value used to initialize every element.
///
/// # Errors
///
/// Returns [`std::io::ErrorKind::Other`] if the allocation request fails.
pub(crate) fn create_vec<T>(len: usize, fill: T) -> Result<Vec<T>>
where
    T: Copy,
{
    let mut buffer = Vec::new();
    try_reserve_vec(&mut buffer, len)?;
    buffer.resize(len, fill);
    Ok(buffer)
}
