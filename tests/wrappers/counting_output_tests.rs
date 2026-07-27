// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::CountingOutput`].

use std::io::{
    ErrorKind,
    SeekFrom,
};

use qubit_io::{
    CountingOutput,
    Output,
    Seekable,
};

use super::support_tests::ScriptedOutput;

#[test]
fn test_counting_output_counts_successful_generic_items() {
    let mut output = CountingOutput::new(ScriptedOutput::accepting());
    output.inner_mut().buffered = true;

    assert!(output.is_buffered());
    assert_eq!(0, output.items_written());
    assert_eq!(
        2,
        output.write(&[13_u16, 21]).expect("write should succeed")
    );
    assert_eq!(2, output.items_written());
    assert_eq!(vec![13, 21], output.inner().items);
}

#[test]
fn test_counting_output_exposes_byte_count_and_inner_output() {
    let mut output = CountingOutput::new(Vec::new());
    output.inner_mut().extend_from_slice(b"x");

    assert_eq!(
        2,
        Output::write(&mut output, b"ab").expect("write succeeds")
    );
    assert_eq!(2, output.bytes_written());
    assert_eq!(b"xab", output.into_inner().as_slice());
}

#[test]
fn test_counting_output_preserves_count_on_write_errors_and_invalid_progress() {
    let mut failing =
        CountingOutput::new(ScriptedOutput::<u16>::failing_write("failed"));
    let error = failing
        .write(&[1_u16])
        .expect_err("write error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(0, failing.items_written());

    let mut invalid =
        CountingOutput::new(ScriptedOutput::<u16>::invalid_count());
    let error = invalid
        .write(&[1_u16])
        .expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(0, invalid.items_written());
}

#[test]
fn test_counting_output_forwards_flush_and_seek_without_counting() {
    let mut output = CountingOutput::new(ScriptedOutput::<u16>::accepting());

    output.flush().expect("flush should succeed");
    assert_eq!(1, output.inner().flush_calls);
    assert_eq!(
        4,
        output
            .seek_to(SeekFrom::Start(4))
            .expect("seek should succeed")
    );
    assert_eq!(0, output.items_written());
    assert_eq!(4, output.inner().position);
}
