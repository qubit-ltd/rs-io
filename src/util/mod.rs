// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg
mod allocation;
mod streams;
mod unchecked_slice;

pub(crate) use allocation::{
    allocation_error,
    create_vec,
};
#[cfg(coverage)]
#[doc(hidden)]
pub use allocation::{
    coverage_fail_next_reserve,
    coverage_fail_next_string_reserve,
    coverage_fail_reserve_after,
    coverage_reset_reserve_hooks,
};
pub use allocation::{
    try_reserve_string,
    try_reserve_vec,
};
pub use streams::Streams;
#[cfg(coverage)]
#[doc(hidden)]
pub use streams::{
    coverage_add_item_count_overflow,
    coverage_fail_next_add_item_count,
    coverage_reset_add_item_count_hooks,
};
#[allow(unused_imports)]
pub use unchecked_slice::UncheckedSlice;
