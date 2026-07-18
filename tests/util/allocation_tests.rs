// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::collections::TryReserveError;

use qubit_io::{
    try_reserve_string,
    try_reserve_vec,
};

#[test]
fn test_try_reserve_vec_preserves_try_reserve_error() {
    let mut output = Vec::<u8>::new();

    let error: TryReserveError = try_reserve_vec(&mut output, usize::MAX)
        .expect_err("capacity overflow should return TryReserveError");

    assert!(!error.to_string().is_empty());
    assert!(output.is_empty());
}

#[test]
fn test_try_reserve_string_preserves_try_reserve_error() {
    let mut output = String::new();

    let error: TryReserveError = try_reserve_string(&mut output, usize::MAX)
        .expect_err("capacity overflow should return TryReserveError");

    assert!(!error.to_string().is_empty());
    assert!(output.is_empty());
}
