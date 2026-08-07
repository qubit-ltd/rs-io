// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::CountingInput`].

use std::io::Cursor;
use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::CountingInput;
use qubit_io::Input;
use qubit_io::Seekable;

use super::support_tests::ScriptedInput;

#[test]
fn test_counting_input_counts_successful_generic_items() {
    let mut input = CountingInput::new(ScriptedInput::items(vec![3_u16, 5, 8]));
    input.inner_mut().buffered = true;
    let mut output = [0_u16; 2];

    assert!(input.is_buffered());
    assert_eq!(0, input.items_read());
    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([3, 5], output);
    assert_eq!(2, input.items_read());
    assert_eq!(1, input.inner().remaining_len());
}

#[test]
fn test_counting_input_exposes_byte_count_and_inner_input() {
    let mut input = CountingInput::new(Cursor::new(b"abc".to_vec()));
    input.inner_mut().set_position(1);
    let mut output = [0_u8; 4];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!(b"bc", &output[..2]);
    assert_eq!(2, input.bytes_read());
    assert_eq!(3, input.into_inner().position());
}

#[test]
fn test_counting_input_preserves_count_on_read_errors_and_invalid_progress() {
    let mut failing =
        CountingInput::new(ScriptedInput::<u16>::failing("read failed"));
    let mut output = [0_u16; 2];
    let error = failing
        .read(&mut output)
        .expect_err("read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(0, failing.items_read());

    let mut invalid = CountingInput::new(ScriptedInput::<u16>::invalid_count());
    let error = invalid
        .read(&mut output)
        .expect_err("invalid progress should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(0, invalid.items_read());
}

#[test]
fn test_counting_input_forwards_seek_without_changing_count() {
    let mut input = CountingInput::new(ScriptedInput::items(vec![1_u16]));

    assert_eq!(
        7,
        input
            .seek_to(SeekFrom::Start(7))
            .expect("seek should succeed")
    );
    assert_eq!(0, input.items_read());
    assert_eq!(7, input.inner().position);
}
