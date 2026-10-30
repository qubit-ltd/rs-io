// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::SyncSeekTeeInput`].

use std::io::Cursor;
use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::Input;
use qubit_io::Seekable;
use qubit_io::SyncSeekTeeInput;

use super::support_tests::ErrorBranch;
use super::support_tests::InputAction;
use super::support_tests::ScriptedInput;
use super::support_tests::ScriptedOutput;
use super::support_tests::SeekFailOnceOutput;
use super::support_tests::assert_original_os_error;

#[test]
fn test_sync_seek_tee_input_repeated_error_retains_original_source() {
    let mut input = SyncSeekTeeInput::new(Cursor::new(b"abc"), ErrorBranch::os_error());
    let mut byte = [0];
    assert_eq!(1, input.read(&mut byte).unwrap());
    let error = input.read(&mut byte).unwrap_err();
    assert_original_os_error(&error);
    assert_original_os_error(&input.seek_to(SeekFrom::Start(0)).unwrap_err());
    assert_eq!(1, input.inner().position());
    assert_eq!(1, input.branch().write_calls);
    assert_eq!(0, input.branch().seek_calls);
}

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

    let (source, branch, error) = input.into_parts();
    assert!(error.is_none());
    assert_eq!(0, source.remaining_len());
    assert_eq!(vec![10, 20], branch.items);
}

#[test]
fn test_sync_seek_tee_input_mutable_accessors_modify_both_paths() {
    let mut input = SyncSeekTeeInput::new(ScriptedInput::items(vec![1_u16]), ScriptedOutput::accepting());
    input.inner_mut().action = InputAction::Items(vec![2]);
    input.branch_mut().items.push(1);
    let mut output = [0_u16; 1];

    assert_eq!(1, input.read(&mut output).expect("read should succeed"));
    assert_eq!([2], output);
    assert_eq!(vec![1, 2], input.branch().items);
}

#[test]
fn test_sync_seek_tee_input_empty_read_does_not_touch_healthy_paths() {
    let mut input = SyncSeekTeeInput::new(
        ScriptedInput::<u16>::failing("empty read touched source"),
        ScriptedOutput::<u16>::failing_write("empty read touched branch"),
    );
    assert_eq!(0, input.read(&mut []).expect("empty read should succeed"));
    assert_eq!(0, input.inner().position);
    assert_eq!(0, input.branch().position);
    assert!(input.branch().items.is_empty());
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
    assert_eq!(
        2,
        branch_failure
            .read(&mut output)
            .expect("primary progress should be returned")
    );
    assert_eq!(ErrorKind::Other, branch_failure.branch_error().unwrap().kind());
    assert_eq!([1, 2], output);
    assert_eq!(0, branch_failure.inner().remaining_len());
    let source_position = branch_failure.inner().position;
    let branch_position = branch_failure.branch().position;
    branch_failure.inner_mut().action = InputAction::Error("empty read touched source");
    assert_eq!(
        0,
        branch_failure
            .read(&mut [])
            .expect("empty read should succeed after failure")
    );
    assert_eq!(source_position, branch_failure.inner().position);
    assert_eq!(branch_position, branch_failure.branch().position);
    assert_eq!(
        ErrorKind::Other,
        branch_failure
            .seek_to(SeekFrom::Start(42))
            .expect_err("seek after branch failure should be rejected")
            .kind()
    );
    assert_eq!(source_position, branch_failure.inner().position);
    assert_eq!(branch_position, branch_failure.branch().position);
    assert_eq!(ErrorKind::Other, branch_failure.read(&mut output).unwrap_err().kind());
    assert_eq!(0, branch_failure.inner().remaining_len());
}

#[test]
fn test_sync_seek_tee_input_std_read_preserves_primary_progress() {
    let tee = SyncSeekTeeInput::new(Cursor::new(&b"abcd"[..]), FailingSeekableOutput);
    let mut reader = qubit_io::StdRead::new(tee);
    let mut bytes = [0; 3];
    assert_eq!(3, std::io::Read::read(&mut reader, &mut bytes).unwrap());
    assert_eq!(&bytes, b"abc");
    assert_eq!(
        ErrorKind::WriteZero,
        std::io::Read::read(&mut reader, &mut [0]).unwrap_err().kind()
    );
    assert_eq!(3, reader.get_ref().inner().position());
}

struct FailingSeekableOutput;

impl qubit_io::Output for FailingSeekableOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count)
    }

    fn write_fully(&mut self, _input: &[u8]) -> std::io::Result<()> {
        Err(ErrorKind::WriteZero.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seekable for FailingSeekableOutput {
    type Unit = u8;

    fn seek_to(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

#[test]
fn test_sync_seek_tee_input_seeks_source_then_aligns_branch() {
    let mut input = SyncSeekTeeInput::new(ScriptedInput::items(vec![1_u16]), ScriptedOutput::accepting());
    assert_eq!(5, input.seek_to(SeekFrom::Start(5)).expect("seek should succeed"));
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

#[test]
fn test_sync_seek_tee_input_seek_failure_recovery_contract() {
    let branch = SeekFailOnceOutput {
        inner: Cursor::new(vec![b'.'; 6]),
        fail_next_seek: true,
    };
    let mut input = SyncSeekTeeInput::new(Cursor::new(b"abcdef".to_vec()), branch);
    assert_eq!(ErrorKind::Other, input.seek_to(SeekFrom::Start(2)).unwrap_err().kind());
    assert!(input.branch_error().is_none());
    assert_eq!(2, input.inner().position());
    assert_eq!(0, input.branch().inner.position());
    let mut byte = [0];
    input
        .read_exactly(&mut byte)
        .expect("existing behavior allows continued reads");
    assert_eq!(b"c", &byte);
    assert_eq!(3, input.inner().position());
    assert_eq!(1, input.branch().inner.position());
    assert_eq!(b"c.....", input.branch().inner.get_ref().as_slice());
    assert!(input.branch_error().is_none());
    assert_eq!(
        4,
        input.seek_to(SeekFrom::Start(4)).expect("resynchronization succeeds")
    );
    assert_eq!(4, input.inner().position());
    assert_eq!(4, input.branch().inner.position());
    input.read_exactly(&mut byte).expect("aligned read succeeds");
    assert_eq!(b"e", &byte);
    assert_eq!(5, input.inner().position());
    assert_eq!(5, input.branch().inner.position());
    assert_eq!(b"c...e.", input.branch().inner.get_ref().as_slice());
    assert!(input.branch_error().is_none());
}

#[test]
fn test_sync_seek_tee_input_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::SyncSeekTeeInput::new(
        super::support_tests::SafeOverreportInput,
        super::support_tests::ScriptedOutput::<u8>::accepting(),
    );
    let mut bytes = [0_u8; 2];
    let error = qubit_io::Input::read(&mut wrapper, &mut bytes).unwrap_err();
    assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
    assert!(wrapper.branch().items.is_empty());
}
