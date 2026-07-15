// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use core::num::NonZeroUsize;

use qubit_io::{nz, nz_const};

#[test]
fn nz_returns_non_zero() {
    assert_eq!(nz(1).get(), 1);
    assert_eq!(nz(42).get(), 42);
}

#[test]
#[should_panic(expected = "must be non-zero")]
fn nz_zero_panics() {
    let _ = nz(0);
}

#[test]
fn nz_macro_in_const_position() {
    const VALUE: NonZeroUsize = nz_const(7);
    assert_eq!(VALUE.get(), 7);
}

#[test]
fn nz_const_with_non_zero_value_uses_runtime_path() {
    let value = nz_const(13);
    assert_eq!(13, value.get());
}

#[test]
#[should_panic(expected = "must be non-zero")]
fn nz_const_zero_panics() {
    let _ = nz_const(0);
}
