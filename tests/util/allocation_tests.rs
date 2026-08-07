// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(coverage)]
use qubit_io::{
    coverage_fail_next_reserve,
    coverage_fail_next_string_reserve,
    coverage_fail_reserve_above,
    coverage_fail_reserve_after,
    coverage_reset_reserve_hooks,
};

/// Verifies that allocation coverage hooks can be configured and reset.
#[cfg(coverage)]
#[test]
fn test_allocation_coverage_hooks_can_be_reset() {
    coverage_fail_next_reserve();
    coverage_fail_next_string_reserve();
    coverage_fail_reserve_above(1);
    coverage_fail_reserve_after(1);
    coverage_reset_reserve_hooks();
}
