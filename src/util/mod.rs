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

pub(crate) use allocation::allocation_error;
#[cfg(coverage)]
pub use allocation::coverage_fail_next_reserve;
#[cfg(coverage)]
pub use allocation::coverage_fail_next_string_reserve;
#[cfg(coverage)]
pub use allocation::coverage_fail_reserve_above;
#[cfg(coverage)]
pub use allocation::coverage_fail_reserve_after;
#[cfg(coverage)]
pub use allocation::coverage_reset_reserve_hooks;
pub(crate) use allocation::create_vec;
pub(crate) use allocation::try_reserve_string;
pub(crate) use allocation::try_reserve_vec;
pub(crate) use qubit_utils::SliceRange;
pub(crate) use qubit_utils::UncheckedSlice;
pub use streams::Streams;
#[cfg(coverage)]
#[doc(hidden)]
pub use streams::coverage_add_item_count_overflow;
#[cfg(coverage)]
#[doc(hidden)]
pub use streams::coverage_fail_next_add_item_count;
#[cfg(coverage)]
#[doc(hidden)]
pub use streams::coverage_reset_add_item_count_hooks;
