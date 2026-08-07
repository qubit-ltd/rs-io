// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Allocation helpers owned by `qubit-io`.

// qubit-style: allow coverage-cfg
use std::collections::TryReserveError;

use qubit_utils::{
    allocation_error as qubit_allocation_error,
    try_reserve_string as qubit_try_reserve_string,
    try_reserve_vec as qubit_try_reserve_vec,
};

#[cfg(coverage)]
use std::cell::Cell;

#[cfg(coverage)]
thread_local! {
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_RESERVE_MAX_ADDITIONAL: Cell<usize> = const { Cell::new(usize::MAX) };
    static COVERAGE_FAIL_NEXT_STRING_RESERVE: Cell<bool> = const { Cell::new(false) };
}

/// Converts a reserve error into an I/O allocation error.
pub(crate) fn allocation_error(error: TryReserveError) -> std::io::Error {
    qubit_allocation_error(error)
}

/// Creates a vector with `len` cloned elements using the local reserve path.
pub(crate) fn create_vec<T>(len: usize, fill: T) -> std::io::Result<Vec<T>>
where
    T: Clone,
{
    let mut buffer = Vec::new();
    try_reserve_vec(&mut buffer, len).map_err(allocation_error)?;
    buffer.resize(len, fill);
    Ok(buffer)
}

/// Reserves vector capacity, applying local coverage failure injection first.
pub(crate) fn try_reserve_vec<T>(
    output: &mut Vec<T>,
    additional: usize,
) -> Result<(), TryReserveError> {
    #[cfg(coverage)]
    if let Some(result) = coverage_maybe_fail_reserve::<()>(additional) {
        return result;
    }
    qubit_try_reserve_vec(output, additional)
}

/// Reserves string capacity, applying local coverage failure injection first.
pub(crate) fn try_reserve_string(
    output: &mut String,
    additional: usize,
) -> Result<(), TryReserveError> {
    #[cfg(coverage)]
    if COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| {
        let fail = state.get();
        if fail {
            state.set(false);
        }
        fail
    }) {
        return Err(coverage_reserve_error());
    }
    #[cfg(coverage)]
    if let Some(result) = coverage_maybe_fail_reserve::<()>(additional) {
        return result;
    }
    qubit_try_reserve_string(output, additional)
}

/// Makes the next vector reserve fail in coverage builds.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_fail_next_reserve() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(0));
}

/// Makes a later vector reserve fail after successful attempts.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_fail_reserve_after(successful_attempts: usize) {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(successful_attempts));
}

/// Makes vector reserves above `max_additional` fail in coverage builds.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_fail_reserve_above(max_additional: usize) {
    COVERAGE_RESERVE_MAX_ADDITIONAL.with(|state| state.set(max_additional));
}

/// Makes the next string reserve fail in coverage builds.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_fail_next_string_reserve() {
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(true));
}

/// Resets all local allocation coverage hooks.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_reset_reserve_hooks() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
    COVERAGE_RESERVE_MAX_ADDITIONAL.with(|state| state.set(usize::MAX));
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(false));
}

#[cfg(coverage)]
fn coverage_reserve_error() -> TryReserveError {
    Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("reserving usize::MAX bytes must exceed Vec capacity")
}

#[cfg(coverage)]
fn coverage_maybe_fail_reserve<T>(
    additional: usize,
) -> Option<Result<T, TryReserveError>> {
    if COVERAGE_RESERVE_MAX_ADDITIONAL.with(|state| additional > state.get()) {
        return Some(Err(coverage_reserve_error()));
    }
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| {
        let remaining = state.get();
        if remaining == usize::MAX {
            return None;
        }
        if remaining == 0 {
            state.set(usize::MAX);
            return Some(Err(coverage_reserve_error()));
        }
        state.set(remaining - 1);
        None
    })
}
