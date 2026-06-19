// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg
mod allocation;
mod nz;
mod streams;
mod unchecked_slice;

pub(crate) use allocation::create_vec;
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
#[allow(unused_imports)]
pub use nz::{
    nz,
    nz_const,
};
pub use streams::Streams;
#[allow(unused_imports)]
pub use unchecked_slice::UncheckedSlice;
