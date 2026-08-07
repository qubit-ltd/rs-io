// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::DEFAULT_BUFFER_CAPACITY;
use qubit_io::DEFAULT_COMPARE_BUFFER_SIZE;
use qubit_io::DEFAULT_COPY_BUFFER_SIZE;

#[test]
fn test_default_capacities_match_documented_sizes() {
    assert_eq!(8 * 1024, DEFAULT_BUFFER_CAPACITY);
    assert_eq!(16 * 1024, DEFAULT_COMPARE_BUFFER_SIZE);
    assert_eq!(16 * 1024, DEFAULT_COPY_BUFFER_SIZE);
}
