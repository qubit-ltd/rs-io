// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{Error, ErrorKind};

use qubit_io::Output;

enum WriteStep {
    Accept(usize),
    Interrupted,
    Error(ErrorKind, &'static str),
    Zero,
}

struct ScriptedOutput {
    values: Vec<u16>,
    steps: VecDeque<WriteStep>,
}

impl ScriptedOutput {
    fn new(steps: Vec<WriteStep>) -> Self {
        Self {
            values: Vec::new(),
            steps: VecDeque::from(steps),
        }
    }
}

impl Output for ScriptedOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        match self.steps.pop_front().unwrap_or(WriteStep::Accept(count)) {
            WriteStep::Accept(max_count) => {
                let written = count.min(max_count);
                self.values
                    .extend_from_slice(&input[index..index + written]);
                Ok(written)
            }
            WriteStep::Interrupted => Err(Error::new(ErrorKind::Interrupted, "interrupted")),
            WriteStep::Error(kind, message) => Err(Error::new(kind, message)),
            WriteStep::Zero => Ok(0),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct OverreportingOutput;

impl Output for OverreportingOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        _input: &[u16],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_output_default_reports_unbuffered() {
    let output = ScriptedOutput::new(vec![]);

    assert!(!output.is_buffered());
}

#[test]
fn test_output_write_returns_successful_count() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Accept(2)]);

    let written = output
        .write(&[10, 20, 30])
        .expect("default write should return successful short count");

    assert_eq!(2, written);
    assert_eq!(&[10, 20], output.values.as_slice());
}

#[test]
fn test_output_write_propagates_implementation_error() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Error(
        ErrorKind::BrokenPipe,
        "write failed",
    )]);

    let error = output
        .write(&[10, 20, 30])
        .expect_err("default write should propagate implementation errors");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
}

#[test]
fn test_output_write_fully_writes_until_range_is_complete() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Accept(2), WriteStep::Accept(2)]);
    let input = [10, 20, 30, 40, 50];

    // SAFETY: `input[1..5]` is a valid source range.
    unsafe {
        output
            .write_fully_unchecked(&input, 1, 4)
            .expect("write_fully should finish after partial writes");
    }

    assert_eq!(&[20, 30, 40, 50], output.values.as_slice());
}

#[test]
fn test_output_write_fully_writes_full_slice() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Accept(1), WriteStep::Accept(2)]);

    output
        .write_fully(&[1, 2, 3])
        .expect("default write_fully should finish after partial writes");

    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_output_write_fully_retries_interrupted_writes() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Interrupted, WriteStep::Accept(3)]);

    // SAFETY: The full input range is valid.
    unsafe {
        output
            .write_fully_unchecked(&[1, 2, 3], 0, 3)
            .expect("interrupted writes should be retried");
    }

    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_output_write_fully_returns_write_zero() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Zero]);

    // SAFETY: The full input range is valid.
    let error = unsafe {
        output
            .write_fully_unchecked(&[1, 2, 3], 0, 3)
            .expect_err("zero progress should fail")
    };

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_output_write_fully_returns_non_interrupted_error() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Error(
        ErrorKind::BrokenPipe,
        "write failed",
    )]);

    // SAFETY: The full input range is valid.
    let error = unsafe {
        output
            .write_fully_unchecked(&[1, 2, 3], 0, 3)
            .expect_err("non-interrupted errors should be returned")
    };

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_output_write_fully_rejects_overreported_count() {
    let mut output = OverreportingOutput;

    // SAFETY: The full input range is valid.
    let error = unsafe {
        output
            .write_fully_unchecked(&[1, 2, 3], 0, 3)
            .expect_err("overreported write count should be rejected")
    };

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_output_write_rejects_overreported_count() {
    let mut output = OverreportingOutput;

    let error = output
        .write(&[1, 2, 3])
        .expect_err("overreported write count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
