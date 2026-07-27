// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::SyncSeekTeeInput`].

use std::io::{
    ErrorKind,
    SeekFrom,
};

use qubit_io::{
    Input,
    Seekable,
    SyncSeekTeeInput,
};

use super::support_tests::{
    InputAction,
    ScriptedInput,
    ScriptedOutput,
};

#[test]
fn test_sync_seek_tee_input_mirrors_generic_items_and_exposes_parts() {
    let mut source = ScriptedInput::items(vec![10_u16, 20]);
    source.buffered = true;
    let branch = ScriptedOutput::accepting();
    let mut input = SyncSeekTeeInput::new(source, branch);
    let mut output = [0_u16; 2];

    assert!(input.is_buffered());
    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([10, 20], output);
    assert_eq!(vec![10, 20], input.branch().items);

    let (source, branch) = input.into_parts();
    assert_eq!(0, source.remaining_len());
    assert_eq!(vec![10, 20], branch.items);
}

#[test]
fn test_sync_seek_tee_input_mutable_accessors_modify_both_paths() {
    let mut input = SyncSeekTeeInput::new(
        ScriptedInput::items(vec![1_u16]),
        ScriptedOutput::accepting(),
    );
    input.inner_mut().action = InputAction::Items(vec![2]);
    input.branch_mut().items.push(1);
    let mut output = [0_u16; 1];

    assert_eq!(1, input.read(&mut output).expect("read should succeed"));
    assert_eq!([2], output);
    assert_eq!(vec![1, 2], input.branch().items);
}

#[test]
fn test_sync_seek_tee_input_propagates_read_path_errors() {
    let mut source_failure = SyncSeekTeeInput::new(
        ScriptedInput::<u16>::failing("source failed"),
        ScriptedOutput::accepting(),
    );
    let mut output = [0_u16; 2];
    let error = source_failure
        .read(&mut output)
        .expect_err("source error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert!(source_failure.branch().items.is_empty());

    let mut branch_failure = SyncSeekTeeInput::new(
        ScriptedInput::items(vec![1_u16, 2]),
        ScriptedOutput::failing_write("branch failed"),
    );
    let error = branch_failure
        .read(&mut output)
        .expect_err("branch error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!([1, 2], output);
    assert_eq!(0, branch_failure.inner().remaining_len());
}

#[test]
fn test_sync_seek_tee_input_seeks_source_then_aligns_branch() {
    let mut input = SyncSeekTeeInput::new(
        ScriptedInput::items(vec![1_u16]),
        ScriptedOutput::accepting(),
    );
    assert_eq!(
        5,
        input
            .seek_to(SeekFrom::Start(5))
            .expect("seek should succeed")
    );
    assert_eq!(5, input.inner().position);
    assert_eq!(5, input.branch().position);

    input.inner_mut().seek_error = Some("source seek failed");
    let error = input
        .seek_to(SeekFrom::Start(7))
        .expect_err("source seek error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(5, input.branch().position);

    input.inner_mut().seek_error = None;
    input.branch_mut().seek_error = Some("branch seek failed");
    let error = input
        .seek_to(SeekFrom::Start(9))
        .expect_err("branch seek error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(9, input.inner().position);
    assert_eq!(5, input.branch().position);
}
