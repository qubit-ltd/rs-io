// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{Cursor, Error, ErrorKind, Read};

use qubit_io::Input;

struct OverreportingInput;

struct OverreportingStdReader;

struct FailingStdReader;

impl Read for FailingStdReader {
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::PermissionDenied, "read failed"))
    }
}

impl Read for OverreportingStdReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        Ok(output.len() + 1)
    }
}

impl Input for OverreportingInput {
    type Item = u8;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u8],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
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
}

impl ScriptedInput {
    fn new(steps: Vec<ReadStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
        }
    }
}

impl Input for ScriptedInput {
    type Item = u8;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        match self.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(data) => {
                let read = count.min(data.len());
                output[index..index + read].copy_from_slice(&data[..read]);
                Ok(read)
            }
            ReadStep::Interrupted => Err(Error::new(ErrorKind::Interrupted, "interrupted")),
            ReadStep::Error(kind, message) => Err(Error::new(kind, message)),
            ReadStep::Eof => Ok(0),
        }
    }
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
    let mut input = ScriptedInput::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "read failed",
    )]);
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
    let mut input = ScriptedInput::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "read failed",
    )]);
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

#[test]
fn test_read_blanket_impl_exposes_input_read_and_read_unchecked() {
    let mut cursor = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 4];

    let read = Input::read(&mut cursor, &mut output).expect("read should succeed");
    assert_eq!(2, read);
    assert_eq!(b"ab\x00\x00", &output);

    let mut cursor = Cursor::new(b"cd".to_vec());
    let mut output = [b'.'; 4];
    // SAFETY: `output[1..3]` is a valid destination range.
    let read = unsafe {
        Input::read_unchecked(&mut cursor, &mut output, 1, 2)
            .expect("read_unchecked should succeed")
    };
    assert_eq!(2, read);
    assert_eq!(b".cd.", &output);
}

#[test]
fn test_read_blanket_impl_rejects_overreported_count() {
    let mut reader = OverreportingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output)
        .expect_err("blanket input should validate the Read count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_blanket_impl_propagates_std_read_error() {
    let mut reader = FailingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output)
        .expect_err("blanket input should propagate Read errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}
