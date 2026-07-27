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
use std::io::{Error, ErrorKind, Result};

/// Converts a fallible allocation error into an I/O error.
///
/// # Parameters
///
/// - `error`: Allocation failure reported by a collection.
///
/// # Returns
///
/// Returns an [`ErrorKind::OutOfMemory`] error that preserves the allocation
/// error as its source.
#[inline]
#[must_use]
pub(crate) fn allocation_error(error: TryReserveError) -> Error {
    Error::new(ErrorKind::OutOfMemory, error)
}

#[cfg(coverage)]
thread_local! {
    /// Successful vector reserve calls allowed before the injected failure.
    static COVERAGE_RESERVE_FAIL_AFTER: Cell<usize> = const { Cell::new(usize::MAX) };
    /// Largest vector reserve request allowed by the coverage hook.
    static COVERAGE_RESERVE_MAX_ADDITIONAL: Cell<usize> = const { Cell::new(usize::MAX) };
    /// Whether the next string reserve request should fail.
    static COVERAGE_FAIL_NEXT_STRING_RESERVE: Cell<bool> = const { Cell::new(false) };
}

/// Makes the next [`try_reserve_vec`] call fail.
///
/// Coverage-only helper for exercising allocation error propagation paths that
/// are impractical to trigger with ordinary test inputs.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_fail_next_reserve() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(0));
}

/// Makes a later reserve call fail after the given number of successful tries.
///
/// A value of `0` fails on the next reserve call. A value of `1` lets one
/// reserve succeed and fails on the following call.
///
/// # Parameters
///
/// - `successful_attempts`: Number of reserve calls allowed to succeed before
///   the injected failure.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_fail_reserve_after(successful_attempts: usize) {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(successful_attempts));
}

/// Fails reserve calls that request more than `max_additional` elements.
///
/// Coverage-only helper for verifying that bounded operations size temporary
/// allocations from their active limits.
///
/// # Parameters
///
/// - `max_additional`: Largest additional capacity request allowed to succeed.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_fail_reserve_above(max_additional: usize) {
    COVERAGE_RESERVE_MAX_ADDITIONAL.with(|state| state.set(max_additional));
}

/// Makes the next [`try_reserve_string`] call fail without affecting vector
/// reserve calls.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_fail_next_string_reserve() {
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(true));
}

/// Clears coverage-only reserve hooks between tests.
#[cfg(coverage)]
#[doc(hidden)]
#[inline]
pub fn coverage_reset_reserve_hooks() {
    COVERAGE_RESERVE_FAIL_AFTER.with(|state| state.set(usize::MAX));
    COVERAGE_RESERVE_MAX_ADDITIONAL.with(|state| state.set(usize::MAX));
    COVERAGE_FAIL_NEXT_STRING_RESERVE.with(|state| state.set(false));
}

/// Creates a deterministic allocation error for coverage-only failure hooks.
///
/// # Returns
///
/// A synthetic capacity-overflow error.
///
/// # Panics
///
/// Panics if reserving `usize::MAX` bytes unexpectedly succeeds.
#[cfg(coverage)]
#[inline]
#[must_use]
fn coverage_reserve_error() -> TryReserveError {
    Vec::<u8>::new()
        .try_reserve(usize::MAX)
        .expect_err("reserving usize::MAX bytes must exceed Vec capacity")
}

/// Returns a synthetic reserve failure when requested by a coverage hook.
///
/// # Type Parameters
///
/// - `T`: Successful result type expected by the calling reserve operation.
///
/// # Parameters
///
/// - `additional`: Additional capacity requested by the calling operation.
///
/// # Returns
///
/// `Some(Err(_))` when a hook injects a failure, or `None` when the real
/// reserve operation should proceed.
#[cfg(coverage)]
fn coverage_maybe_fail_reserve<T>(
    additional: usize,
) -> Option<std::result::Result<T, TryReserveError>> {
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

/// Reserves capacity in a vector without converting allocation failures.
///
/// # Type Parameters
///
/// - `T`: Element type stored by the vector.
///
/// # Parameters
///
/// - `output`: Vector that will receive additional elements.
/// - `additional`: Number of additional elements to reserve.
///
/// # Returns
///
/// `Ok(())` after the requested capacity has been reserved.
///
/// # Errors
///
/// Returns the [`TryReserveError`] reported by [`Vec::try_reserve`] if the
/// allocation request fails or the resulting capacity would overflow.
#[inline]
pub fn try_reserve_vec<T>(
    output: &mut Vec<T>,
    additional: usize,
) -> std::result::Result<(), TryReserveError> {
    #[cfg(coverage)]
    if let Some(result) = coverage_maybe_fail_reserve::<()>(additional) {
        return result;
    }
    output.try_reserve(additional)
}

/// Reserves capacity in a string without converting allocation failures.
///
/// # Parameters
///
/// - `output`: String that will receive additional bytes.
/// - `additional`: Number of additional bytes to reserve.
///
/// # Returns
///
/// `Ok(())` after the requested capacity has been reserved.
///
/// # Errors
///
/// Returns the [`TryReserveError`] reported by [`String::try_reserve`] if the
/// allocation request fails or the resulting capacity would overflow.
#[inline]
pub fn try_reserve_string(
    output: &mut String,
    additional: usize,
) -> std::result::Result<(), TryReserveError> {
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
    output.try_reserve(additional)
}

/// Creates a vector with the requested length and initial value.
///
/// # Type Parameters
///
/// - `T`: Copyable element type stored by the vector.
///
/// # Parameters
///
/// - `len`: Target length of the returned vector.
/// - `fill`: Value used to initialize every element.
///
/// # Returns
///
/// A vector of length `len` whose elements are initialized to `fill`.
///
/// # Errors
///
/// Returns [`ErrorKind::OutOfMemory`] if the allocation request fails or the
/// requested vector length exceeds the supported capacity.
#[inline]
pub(crate) fn create_vec<T>(len: usize, fill: T) -> Result<Vec<T>>
where
    T: Copy,
{
    let mut buffer = Vec::new();
    try_reserve_vec(&mut buffer, len).map_err(allocation_error)?;
    buffer.resize(len, fill);
    Ok(buffer)
}
