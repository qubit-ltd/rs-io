// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::{DEFAULT_BUFFER_CAPACITY, DEFAULT_COMPARE_BUFFER_SIZE, DEFAULT_COPY_BUFFER_SIZE};

#[test]
fn test_default_capacities_are_non_zero() {
    let capacities = std::hint::black_box([
        DEFAULT_BUFFER_CAPACITY,
        DEFAULT_COMPARE_BUFFER_SIZE,
        DEFAULT_COPY_BUFFER_SIZE,
    ]);
    assert!(capacities.into_iter().all(|capacity| capacity > 0));
}
