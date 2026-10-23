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

    let (source, branch, error) = input.into_parts();
    assert!(error.is_none());
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
    let error = input
        .read(&mut output)
        .expect_err("source error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(input.branch().items.is_empty());
}

#[test]
fn test_tee_input_reports_branch_error_after_source_progress() {
    let mut input = TeeInput::new(
        ScriptedInput::items(vec![1_u16, 2]),
        ScriptedOutput::failing_write("branch failed"),
    );
    let mut output = [0_u16; 2];
    assert_eq!(
        2,
        input
            .read(&mut output)
            .expect("primary progress should be returned")
    );

    assert_eq!(ErrorKind::Other, input.branch_error().unwrap().kind());
    assert_eq!([1, 2], output);
    assert_eq!(0, input.inner().remaining_len());
    assert_eq!(
        0,
        input
            .read(&mut [])
            .expect("empty read should succeed after failure")
    );
    assert_eq!(
        ErrorKind::Other,
        input.read(&mut output).unwrap_err().kind()
    );
    assert_eq!(0, input.inner().remaining_len());
    let (_, _, error) = input.into_parts();
    assert!(error.is_some());
}

#[test]
fn test_tee_input_supports_non_copy_items() {
    let mut input = TeeInput::new(
        ScriptedInput::items(vec![String::from("one"), String::from("two")]),
        ScriptedOutput::accepting(),
    );
    let mut output = [String::new(), String::new()];
    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!(output, ["one", "two"]);
    assert_eq!(input.branch().items, ["one", "two"]);
}

#[test]
fn test_tee_input_read_exactly_keeps_primary_success_and_error_cause() {
    let branch = FailingCauseOutput;
    let mut input = TeeInput::new(std::io::Cursor::new(*b"abc"), branch);
    let mut bytes = [0; 3];
    input
        .read_exactly(&mut bytes)
        .expect("exact primary read should succeed");
    assert_eq!(&bytes, b"abc");
    assert_eq!(
        Some(ErrorKind::Other),
        input.branch_error().map(|error| error.kind())
    );
    assert_eq!(3, input.inner().position());
    let (_, _, error) = input.into_parts();
    let error = error.expect("latched branch error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("underlying cause", error.get_ref().unwrap().to_string());
}

struct FailingCauseOutput;

impl qubit_io::Output for FailingCauseOutput {
    type Item = u8;

    unsafe fn write_unchecked(
        &mut self,
        _input: &[u8],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count)
    }

    fn write_fully(&mut self, _input: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::other(UnderlyingCause))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct UnderlyingCause;

impl std::fmt::Display for UnderlyingCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("underlying cause")
    }
}

impl std::error::Error for UnderlyingCause {}

#[test]
fn test_tee_input_rejects_invalid_source_progress() {
    let mut input = TeeInput::new(
        ScriptedInput::<u16>::invalid_count(),
        ScriptedOutput::accepting(),
    );
    let mut output = [0_u16; 2];
    let error = input
        .read(&mut output)
        .expect_err("invalid progress should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(input.branch().items.is_empty());
}

#[test]
fn test_tee_input_forwards_seek_only_to_source() {
    let mut input = TeeInput::new(
        ScriptedInput::items(vec![1_u16]),
        ScriptedOutput::<u16>::accepting(),
    );

    assert_eq!(
        9,
        input
            .seek_to(SeekFrom::Start(9))
            .expect("seek should succeed")
    );
    assert_eq!(9, input.inner().position);
    assert_eq!(0, input.branch().position);
}

#[test]
fn test_tee_input_can_create_synchronized_seek_variant() {
    let mut input = TeeInput::with_sync_branch_seek(
        ScriptedInput::items(vec![1_u16]),
        ScriptedOutput::<u16>::accepting(),
    );

    assert_eq!(
        3,
        input
            .seek_to(SeekFrom::Start(3))
            .expect("seek should succeed")
    );
    assert_eq!(3, input.inner().position);
    assert_eq!(3, input.branch().position);
}

#[test]
fn test_tee_input_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::TeeInput::new(
        super::support_tests::SafeOverreportInput,
        super::support_tests::ScriptedOutput::<u8>::accepting(),
    );
    let mut bytes = [0_u8; 4];
    let error =
        unsafe { qubit_io::Input::read_unchecked(&mut wrapper, &mut bytes, 1, 2) }.unwrap_err();
    assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
    assert!(wrapper.branch().items.is_empty());
}
