// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;

use qubit_io::Input;

struct OverreportingInput;

impl Input for OverreportingInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count + 1)
    }
}

struct FullyOverrideInput {
    reported: usize,
    error: Option<ErrorKind>,
    calls: usize,
}

struct FullyUncheckedOverrideInput {
    reported: usize,
    error: Option<ErrorKind>,
    calls: usize,
}

impl Input for FullyUncheckedOverrideInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
        panic!("read_fully must use the aggregate unchecked override")
    }

    unsafe fn read_fully_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> Result<usize> {
        self.calls += 1;
        if let Some(kind) = self.error {
            return Err(Error::new(kind, "aggregate override error"));
        }
        let actual = self.reported.min(count);
        output[index..index + actual].fill(7);
        Ok(self.reported)
    }
}

#[test]
fn test_read_fully_unchecked_override_rejects_overreported_count() {
    for requested in [0, 2] {
        let mut input = FullyUncheckedOverrideInput {
            reported: requested + 1,
            error: None,
            calls: 0,
        };
        let mut output = vec![0; requested];
        let error = input
            .read_fully(&mut output)
            .expect_err("aggregate overreport must fail");
        assert_eq!(ErrorKind::InvalidData, error.kind(), "requested={requested}");
        assert_eq!(1, input.calls, "aggregate override must be called once");
        assert_eq!(vec![7; requested], output, "validation must not undo writes");
    }
}

#[test]
fn test_read_fully_unchecked_override_preserves_valid_counts() {
    for (requested, reported) in [(0, 0), (2, 0), (2, 1), (2, 2)] {
        let mut input = FullyUncheckedOverrideInput {
            reported,
            error: None,
            calls: 0,
        };
        let mut output = vec![0; requested];
        let actual = input
            .read_fully(&mut output)
            .expect("valid aggregate count must succeed");
        assert_eq!(reported, actual, "requested={requested}");
        assert_eq!(1, input.calls, "empty requests also preserve delegation");
        let mut expected = vec![0; requested];
        expected[..reported].fill(7);
        assert_eq!(expected, output, "only the reported prefix is filled");
    }
}

#[test]
fn test_read_fully_unchecked_override_preserves_errors() {
    for requested in [0, 2] {
        for kind in [ErrorKind::Interrupted, ErrorKind::PermissionDenied] {
            let mut input = FullyUncheckedOverrideInput {
                reported: 0,
                error: Some(kind),
                calls: 0,
            };
            let mut output = vec![0; requested];
            let error = input
                .read_fully(&mut output)
                .expect_err("aggregate error must propagate");
            assert_eq!(kind, error.kind(), "requested={requested}");
            assert_eq!("aggregate override error", error.to_string());
            assert_eq!(1, input.calls, "the safe wrapper must not add retries");
            assert_eq!(vec![0; requested], output);
        }
    }
}

impl Input for FullyOverrideInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
        panic!("read_exactly must use the safe read_fully override")
    }

    fn read_fully(&mut self, output: &mut [u8]) -> Result<usize> {
        self.calls += 1;
        if let Some(kind) = self.error {
            return Err(Error::new(kind, "safe read_fully error"));
        }
        let actual = self.reported.min(output.len());
        output[..actual].fill(7);
        Ok(self.reported)
    }
}

#[test]
fn test_read_exactly_override_rejects_overreported_count() {
    for requested in [0, 2] {
        let mut input = FullyOverrideInput {
            reported: requested + 1,
            error: None,
            calls: 0,
        };
        let mut output = vec![0; requested];
        let error = input.read_exactly(&mut output).expect_err("overreport must fail");
        assert_eq!(ErrorKind::InvalidData, error.kind(), "requested={requested}");
        assert_eq!(1, input.calls, "read_fully must be called exactly once");
        assert_eq!(vec![7; requested], output, "validation does not undo inner writes");
    }
}

#[test]
fn test_read_exactly_override_preserves_valid_results_and_errors() {
    for (requested, reported, expected) in [(0, 0, None), (2, 2, None), (2, 1, Some(ErrorKind::UnexpectedEof))] {
        let mut input = FullyOverrideInput {
            reported,
            error: None,
            calls: 0,
        };
        let mut output = vec![0; requested];
        let result = input.read_exactly(&mut output).map_err(|error| error.kind());
        assert_eq!(expected.map_or(Ok(()), Err), result);
        assert_eq!(1, input.calls);
        assert_eq!(vec![7; reported], output[..reported]);
    }
    for kind in [ErrorKind::Interrupted, ErrorKind::PermissionDenied] {
        let mut input = FullyOverrideInput {
            reported: 0,
            error: Some(kind),
            calls: 0,
        };
        let error = input.read_exactly(&mut [0; 2]).expect_err("inner error must propagate");
        assert_eq!(kind, error.kind());
        assert_eq!("safe read_fully error", error.to_string());
        assert_eq!(1, input.calls, "the wrapper must not add retries");
    }
}

enum ReadStep {
    Data(Vec<u8>),
    Interrupted,
    Error(ErrorKind, &'static str),
    Eof,
}

struct ScriptedInput {
    steps: VecDeque<ReadStep>,
    read_calls: usize,
}

impl ScriptedInput {
    fn new(steps: Vec<ReadStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
            read_calls: 0,
        }
    }
}

impl Input for ScriptedInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> std::io::Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        self.read_calls += 1;
        match self.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(data) => {
                let read = count.min(data.len());
                output[index..index + read].copy_from_slice(&data[..read]);
                if read < data.len() {
                    self.steps.push_front(ReadStep::Data(data[read..].to_vec()));
                }
                Ok(read)
            }
            ReadStep::Interrupted => Err(Error::new(ErrorKind::Interrupted, "interrupted")),
            ReadStep::Error(kind, message) => Err(Error::new(kind, message)),
            ReadStep::Eof => Ok(0),
        }
    }
}

#[test]
fn test_scripted_input_preserves_data_suffix_across_small_reads() {
    let mut input = ScriptedInput::new(vec![ReadStep::Data(vec![1, 2, 3])]);
    let mut output = [0; 1];

    for expected in [1, 2, 3] {
        assert_eq!(1, input.read(&mut output).expect("scripted read should succeed"));
        assert_eq!(expected, output[0]);
        let mut empty = [];
        assert_eq!(0, input.read(&mut empty).expect("zero-length read should succeed"));
    }
    assert_eq!(0, input.read(&mut output).expect("scripted EOF should succeed"));
}

#[test]
fn test_scripted_input_zero_length_read_does_not_consume_data() {
    let mut input = ScriptedInput::new(vec![ReadStep::Data(vec![7])]);
    let mut empty = [];
    // SAFETY: The empty destination and zero count require no memory access; index
    // zero is valid.
    assert_eq!(
        0,
        unsafe { input.read_unchecked(&mut empty, 0, 0) }.expect("empty read should succeed")
    );
    assert_eq!(0, input.read_calls);
    let mut output = [0];
    assert_eq!(1, input.read(&mut output).expect("data should remain available"));
    assert_eq!([7], output);
}

#[test]
fn test_scripted_input_zero_length_read_does_not_consume_error_step() {
    let mut input = ScriptedInput::new(vec![ReadStep::Error(ErrorKind::PermissionDenied, "first")]);
    let mut empty = [];
    // SAFETY: The empty destination and zero count require no memory access; index
    // zero is valid.
    assert_eq!(
        0,
        unsafe { input.read_unchecked(&mut empty, 0, 0) }.expect("empty read should succeed")
    );
    assert_eq!(0, input.read_calls);
    let mut output = [0];
    assert_eq!(
        ErrorKind::PermissionDenied,
        input.read(&mut output).expect_err("error step should remain").kind()
    );
}

#[test]
fn test_input_default_reports_unbuffered() {
    let input = ScriptedInput::new(vec![]);

    assert!(!input.is_buffered());
}

#[test]
fn test_input_read_uses_default_validation() {
    let mut input = OverreportingInput;
    let mut output = [0_u8; 3];

    let error = input
        .read(&mut output)
        .expect_err("default read should validate reported counts");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_input_read_returns_successful_count() {
    let mut input = ScriptedInput::new(vec![ReadStep::Data(vec![1, 2])]);
    let mut output = [0_u8; 4];

    let read = input
        .read(&mut output)
        .expect("default read should return successful short count");

    assert_eq!(2, read);
    assert_eq!([1, 2, 0, 0], output);
}

#[test]
fn test_input_read_propagates_implementation_error() {
    let mut input = ScriptedInput::new(vec![ReadStep::Error(ErrorKind::PermissionDenied, "read failed")]);
    let mut output = [0_u8; 4];

    let error = input
        .read(&mut output)
        .expect_err("default read should propagate implementation errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}

#[test]
fn test_input_read_fully_reads_until_buffer_full_or_eof() {
    let mut input = ScriptedInput::new(vec![
        ReadStep::Data(vec![1, 2]),
        ReadStep::Interrupted,
        ReadStep::Data(vec![3]),
        ReadStep::Eof,
    ]);
    let mut output = [0_u8; 4];

    let read = input
        .read_fully(&mut output)
        .expect("read_fully should return partial count at EOF");

    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0], output);
}

#[test]
fn test_input_read_fully_returns_non_interrupted_error() {
    let mut input = ScriptedInput::new(vec![ReadStep::Error(ErrorKind::PermissionDenied, "read failed")]);
    let mut output = [0_u8; 3];

    let error = input
        .read_fully(&mut output)
        .expect_err("read_fully should return non-interrupted errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_input_read_fully_rejects_overreported_count() {
    let mut input = OverreportingInput;
    let mut output = [0_u8; 3];

    let error = input
        .read_fully(&mut output)
        .expect_err("read_fully should validate reported counts");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_input_read_exactly_fills_destination() {
    let mut input = ScriptedInput::new(vec![
        ReadStep::Data(vec![1, 2]),
        ReadStep::Interrupted,
        ReadStep::Data(vec![3]),
    ]);
    let mut output = [0_u8; 3];

    input
        .read_exactly(&mut output)
        .expect("read_exactly should fill the destination");

    assert_eq!([1, 2, 3], output);
}

#[test]
fn test_input_read_exactly_reports_unexpected_eof() {
    let mut input = ScriptedInput::new(vec![ReadStep::Data(vec![1, 2]), ReadStep::Eof]);
    let mut output = [0_u8; 3];

    let error = input
        .read_exactly(&mut output)
        .expect_err("read_exactly should reject a short read");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!([1, 2, 0], output);
}
