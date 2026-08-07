// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared allocation, stream, and unchecked-slice utilities.

// qubit-style: allow coverage-cfg
mod allocation;
mod streams;

pub(crate) use allocation::{
    allocation_error,
    create_vec,
    try_reserve_string,
    try_reserve_vec,
};
#[cfg(coverage)]
pub use allocation::{
    coverage_fail_next_reserve,
    coverage_fail_next_string_reserve,
    coverage_fail_reserve_above,
    coverage_fail_reserve_after,
    coverage_reset_reserve_hooks,
};
pub(crate) use qubit_utils::{
    SliceRange,
    UncheckedSlice,
};
pub use streams::Streams;
#[cfg(coverage)]
#[doc(hidden)]
pub use streams::{
    coverage_add_item_count_overflow,
    coverage_fail_next_add_item_count,
    coverage_reset_add_item_count_hooks,
};
