// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for vectored-buffer utility contracts.

use crate::util::requested_items;

#[test]
fn test_vectored_lengths_reject_overflow_without_io() {
    let error = requested_items([usize::MAX, 1].into_iter()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}
