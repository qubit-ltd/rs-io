// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::TeeInput`].

use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::Input;
use qubit_io::Seekable;
use qubit_io::TeeInput;

use super::support_tests::InputAction;
use super::support_tests::ScriptedInput;
use super::support_tests::ScriptedOutput;

#[test]
fn test_tee_input_mirrors_generic_items_and_exposes_parts() {
    let mut source = ScriptedInput::items(vec![4_u16, 6, 8]);
    source.buffered = true;
    let branch = ScriptedOutput::short(1);
    let mut input = TeeInput::new(source, branch);
    let mut output = [0_u16; 2];

    assert!(input.is_buffered());
    assert_eq!(3, input.inner().remaining_len());
    assert!(input.branch().items.is_empty());
    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([4, 6], output);
    assert_eq!(vec![4, 6], input.branch().items);

    let (source, branch) = input.into_parts();
    assert_eq!(1, source.remaining_len());
    assert_eq!(vec![4, 6], branch.items);
}

#[test]
fn test_tee_input_mutable_accessors_modify_both_paths() {
    let source = ScriptedInput::items(vec![1_u16]);
    let branch = ScriptedOutput::accepting();
    let mut input = TeeInput::new(source, branch);
    input.inner_mut().action = InputAction::Items(vec![2, 3]);
    input.branch_mut().items.push(1);
    let mut output = [0_u16; 2];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([2, 3], output);
    assert_eq!(vec![1, 2, 3], input.branch().items);
}

#[test]
fn test_tee_input_returns_source_errors_without_writing_branch() {
    let mut input = TeeInput::new(
        ScriptedInput::<u16>::failing("source failed"),
        ScriptedOutput::accepting(),
    );
    let mut output = [0_u16; 2];
    let error = input.read(&mut output).expect_err("source error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(input.branch().items.is_empty());
}

#[test]
fn test_tee_input_returns_branch_error_after_source_progress() {
    let mut input = TeeInput::new(
        ScriptedInput::items(vec![1_u16, 2]),
        ScriptedOutput::failing_write("branch failed"),
    );
    let mut output = [0_u16; 2];
    let error = input.read(&mut output).expect_err("branch error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!([1, 2], output);
    assert_eq!(0, input.inner().remaining_len());
}

#[test]
fn test_tee_input_rejects_invalid_source_progress() {
    let mut input = TeeInput::new(ScriptedInput::<u16>::invalid_count(), ScriptedOutput::accepting());
    let mut output = [0_u16; 2];
    let error = input
        .read(&mut output)
        .expect_err("invalid progress should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(input.branch().items.is_empty());
}

#[test]
fn test_tee_input_forwards_seek_only_to_source() {
    let mut input = TeeInput::new(ScriptedInput::items(vec![1_u16]), ScriptedOutput::<u16>::accepting());

    assert_eq!(9, input.seek_to(SeekFrom::Start(9)).expect("seek should succeed"));
    assert_eq!(9, input.inner().position);
    assert_eq!(0, input.branch().position);
}

#[test]
fn test_tee_input_can_create_synchronized_seek_variant() {
    let mut input =
        TeeInput::with_sync_branch_seek(ScriptedInput::items(vec![1_u16]), ScriptedOutput::<u16>::accepting());

    assert_eq!(3, input.seek_to(SeekFrom::Start(3)).expect("seek should succeed"));
    assert_eq!(3, input.inner().position);
    assert_eq!(3, input.branch().position);
}
