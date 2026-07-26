// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::LimitInput`].

use std::io::ErrorKind;

use qubit_io::{
    Input,
    LimitInput,
};

use super::support_tests::ScriptedInput;

#[test]
fn test_limit_input_reads_at_most_the_remaining_generic_items() {
    let mut input = LimitInput::new(ScriptedInput::items(vec![1_u16, 2, 3]), 2);
    input.inner_mut().buffered = true;
    let mut items = [0_u16; 3];

    assert!(input.is_buffered());
    assert_eq!(2, input.remaining());
    assert_eq!(2, input.read(&mut items).expect("read should succeed"));
    assert_eq!([1, 2, 0], items);
    assert_eq!(0, input.remaining());
    assert_eq!(1, input.inner().remaining_len());
    assert_eq!(0, input.read(&mut items).expect("limit acts as EOF"));
    assert_eq!(1, input.into_inner().remaining_len());
}

#[test]
fn test_limit_input_zero_count_does_not_call_inner_input() {
    let mut input = LimitInput::new(ScriptedInput::<u16>::failing("unused"), 1);
    let mut items = [];

    assert_eq!(0, input.read(&mut items).expect("empty read succeeds"));
    assert_eq!(1, input.remaining());
}

#[test]
fn test_limit_input_preserves_remaining_on_error_and_invalid_progress() {
    let mut failing =
        LimitInput::new(ScriptedInput::<u16>::failing("read failed"), 3);
    let mut items = [0_u16; 2];
    let error = failing
        .read(&mut items)
        .expect_err("read error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(3, failing.remaining());

    let mut invalid = LimitInput::new(ScriptedInput::<u16>::invalid_count(), 2);
    let error = invalid
        .read(&mut items)
        .expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(2, invalid.remaining());
}
