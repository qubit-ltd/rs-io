// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::LimitOutput`].

use std::io::ErrorKind;

use qubit_io::{LimitOutput, Output};

use super::support_tests::ScriptedOutput;

#[test]
fn test_limit_output_writes_at_most_the_remaining_generic_items() {
    let mut output = LimitOutput::new(ScriptedOutput::accepting(), 2);
    output.inner_mut().buffered = true;

    assert!(output.is_buffered());
    assert_eq!(2, output.remaining());
    assert_eq!(
        2,
        output.write(&[1_u16, 2, 3]).expect("write should succeed")
    );
    assert_eq!(0, output.remaining());
    assert_eq!(vec![1, 2], output.inner().items);
    assert_eq!(0, output.write(&[4]).expect("limit returns zero"));
    assert_eq!(vec![1, 2], output.into_inner().items);
}

#[test]
fn test_limit_output_zero_count_does_not_call_inner_output() {
    let mut output = LimitOutput::new(ScriptedOutput::<u16>::failing_write("unused"), 1);

    assert_eq!(0, output.write(&[]).expect("empty write succeeds"));
    assert_eq!(1, output.remaining());
}

#[test]
fn test_limit_output_preserves_remaining_on_error_and_invalid_progress() {
    let mut failing = LimitOutput::new(ScriptedOutput::<u16>::failing_write("failed"), 3);
    let error = failing
        .write(&[1_u16, 2])
        .expect_err("write error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(3, failing.remaining());

    let mut invalid = LimitOutput::new(ScriptedOutput::<u16>::invalid_count(), 2);
    let error = invalid
        .write(&[1_u16, 2])
        .expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(2, invalid.remaining());
}

#[test]
fn test_limit_output_forwards_flush() {
    let mut output = LimitOutput::new(ScriptedOutput::<u16>::accepting(), 2);

    output.flush().expect("flush should succeed");
    assert_eq!(1, output.inner().flush_calls);
}
