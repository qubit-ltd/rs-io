// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::UncheckedSlice;

#[test]
fn test_unchecked_slice_range_validation_rejects_overflow() {
    assert!(!UncheckedSlice::range_fits(4, usize::MAX, 1));
}
