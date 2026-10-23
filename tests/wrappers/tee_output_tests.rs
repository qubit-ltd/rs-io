// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::TeeOutput`].

use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::BufferedOutput;
use qubit_io::CountingOutput;
use qubit_io::LimitOutput;
use qubit_io::Output;
use qubit_io::Seekable;
use qubit_io::TeeOutput;

use super::support_tests::ScriptedOutput;

#[test]
fn test_tee_output_mirrors_primary_prefix_and_exposes_parts() {
    let primary = ScriptedOutput::short(2);
    let branch = ScriptedOutput::short(1);
    let mut output = TeeOutput::new(primary, branch);

    assert!(!output.is_buffered());
    assert_eq!(
        2,
        output.write(&[7_u16, 9, 11]).expect("write should succeed")
    );
    assert_eq!(vec![7, 9], output.inner().items);
    assert_eq!(vec![7, 9], output.branch().items);

    let (primary, branch, error) = output.into_parts();
    assert!(error.is_none());
    assert_eq!(vec![7, 9], primary.items);
    assert_eq!(vec![7, 9], branch.items);
}

#[test]
fn test_tee_output_is_buffered_only_when_both_paths_are_buffered() {
    let mut output = TeeOutput::new(
        ScriptedOutput::<u16>::accepting(),
        ScriptedOutput::accepting(),
    );
    output.inner_mut().buffered = true;
    assert!(!output.is_buffered());
    output.branch_mut().buffered = true;
    assert!(output.is_buffered());
}

#[test]
fn test_tee_output_mutable_accessors_modify_both_paths() {
    let mut output = TeeOutput::new(ScriptedOutput::accepting(), ScriptedOutput::accepting());
    output.inner_mut().items.push(1_u16);
    output.branch_mut().items.push(2);

    assert_eq!(1, output.write(&[3]).expect("write should succeed"));
    assert_eq!(vec![1, 3], output.inner().items);
    assert_eq!(vec![2, 3], output.branch().items);
}

#[test]
fn test_tee_output_returns_primary_errors_without_writing_branch() {
    let mut output = TeeOutput::new(
        ScriptedOutput::<u16>::failing_write("primary failed"),
        ScriptedOutput::accepting(),
    );
    let error = output
        .write(&[1])
        .expect_err("primary error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(output.branch().items.is_empty());

    let mut invalid = TeeOutput::new(
        ScriptedOutput::<u16>::invalid_count(),
        ScriptedOutput::accepting(),
    );
    let error = invalid
        .write(&[1])
        .expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(invalid.branch().items.is_empty());
}

#[test]
fn test_tee_output_reports_branch_error_after_primary_progress() {
    let mut output = TeeOutput::new(
        ScriptedOutput::accepting(),
        ScriptedOutput::<u16>::failing_write("branch failed"),
    );
    assert_eq!(
        2,
        output
            .write(&[1_u16, 2])
            .expect("primary progress should be returned")
    );

    assert_eq!(ErrorKind::Other, output.branch_error().unwrap().kind());
    assert_eq!(vec![1, 2], output.inner().items);
    assert_eq!(ErrorKind::Other, output.write(&[3]).unwrap_err().kind());
    assert_eq!(vec![1, 2], output.inner().items);
    assert_eq!(ErrorKind::Other, output.flush().unwrap_err().kind());
    assert_eq!(0, output.inner().flush_calls);
    let (_, _, error) = output.into_parts();
    assert!(error.is_some());
}

#[test]
fn test_tee_output_composes_with_counting_and_buffering() {
    let tee = TeeOutput::new(Vec::<u8>::new(), LimitOutput::new(Vec::<u8>::new(), 2));
    let mut counted = CountingOutput::new(tee);
    assert_eq!(
        3,
        counted.write(b"abc").expect("primary write should succeed")
    );
    assert_eq!(3, counted.items_written());
    assert_eq!(
        Some(ErrorKind::WriteZero),
        counted.inner().branch_error().map(|error| error.kind())
    );

    let tee = TeeOutput::new(Vec::<u8>::new(), LimitOutput::new(Vec::<u8>::new(), 2));
    let mut buffered = BufferedOutput::with_capacity(tee, 3);
    buffered
        .write_fully(b"abc")
        .expect("buffer should accept input");
    assert_eq!(ErrorKind::WriteZero, buffered.flush().unwrap_err().kind());
    assert_eq!(b"abc", buffered.inner_mut().inner().as_slice());
    assert_eq!(ErrorKind::WriteZero, buffered.flush().unwrap_err().kind());
    assert_eq!(b"abc", buffered.inner_mut().inner().as_slice());
}

#[test]
fn test_tee_output_write_fully_latches_interrupted_without_retrying() {
    let mut output = TeeOutput::new(Vec::<u8>::new(), InterruptedFullyOutput);
    assert_eq!(
        3,
        output
            .write(b"abc")
            .expect("primary progress should be returned")
    );
    assert_eq!(
        Some(ErrorKind::Interrupted),
        output.branch_error().map(|error| error.kind())
    );
    assert_eq!(ErrorKind::Other, output.write(b"d").unwrap_err().kind());
    assert_eq!(b"abc", output.inner().as_slice());
}

#[test]
fn test_tee_output_write_fully_success_still_exposes_branch_failure() {
    let mut output = TeeOutput::new(Vec::<u8>::new(), LimitOutput::new(Vec::<u8>::new(), 2));
    output
        .write_fully(b"abc")
        .expect("primary full write should succeed");
    assert_eq!(b"abc", output.inner().as_slice());
    assert_eq!(
        Some(ErrorKind::WriteZero),
        output.branch_error().map(|error| error.kind())
    );
}

struct InterruptedFullyOutput;

impl Output for InterruptedFullyOutput {
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
        Err(ErrorKind::Interrupted.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_tee_output_does_not_touch_branch_after_zero_primary_progress() {
    let mut output = TeeOutput::new(
        ScriptedOutput::short(0),
        ScriptedOutput::<u16>::failing_write("must not run"),
    );

    assert_eq!(0, output.write(&[1_u16]).expect("zero write succeeds"));
    assert!(output.branch().items.is_empty());
}

#[test]
fn test_tee_output_flushes_in_primary_then_branch_order() {
    let mut success = TeeOutput::new(
        ScriptedOutput::<u16>::accepting(),
        ScriptedOutput::accepting(),
    );
    success.flush().expect("flush should succeed");
    assert_eq!(1, success.inner().flush_calls);
    assert_eq!(1, success.branch().flush_calls);

    let mut primary_failure = TeeOutput::new(
        ScriptedOutput::<u16>::failing_flush("primary flush failed"),
        ScriptedOutput::accepting(),
    );
    let error = primary_failure
        .flush()
        .expect_err("primary flush error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(0, primary_failure.branch().flush_calls);

    let mut branch_failure = TeeOutput::new(
        ScriptedOutput::<u16>::accepting(),
        ScriptedOutput::failing_flush("branch flush failed"),
    );
    let error = branch_failure
        .flush()
        .expect_err("branch flush error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(1, branch_failure.inner().flush_calls);
    assert_eq!(1, branch_failure.branch().flush_calls);
}

#[test]
fn test_tee_output_seeks_primary_then_aligns_branch() {
    let mut output = TeeOutput::new(
        ScriptedOutput::<u16>::accepting(),
        ScriptedOutput::accepting(),
    );
    assert_eq!(
        7,
        output
            .seek_to(SeekFrom::Start(7))
            .expect("seek should succeed")
    );
    assert_eq!(7, output.inner().position);
    assert_eq!(7, output.branch().position);

    output.inner_mut().seek_error = Some("primary seek failed");
    let error = output
        .seek_to(SeekFrom::Start(9))
        .expect_err("primary seek error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(7, output.branch().position);

    output.inner_mut().seek_error = None;
    output.branch_mut().seek_error = Some("branch seek failed");
    let error = output
        .seek_to(SeekFrom::Start(11))
        .expect_err("branch seek error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(11, output.inner().position);
    assert_eq!(7, output.branch().position);
}

#[test]
fn test_tee_output_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::TeeOutput::new(
        super::support_tests::SafeOverreportOutput,
        super::support_tests::ScriptedOutput::<u8>::accepting(),
    );
    let error =
        unsafe { qubit_io::Output::write_unchecked(&mut wrapper, b"abcd", 1, 2) }.unwrap_err();
    assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
    assert!(wrapper.branch().items.is_empty());
}
