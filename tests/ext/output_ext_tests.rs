// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{
    Error,
    ErrorKind,
};

use qubit_io::{
    Output,
    OutputExt,
};

enum WriteStep {
    Accept(usize),
    Interrupted,
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
            WriteStep::Interrupted => {
                Err(Error::new(ErrorKind::Interrupted, "interrupted"))
            }
            WriteStep::Zero => Ok(0),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_output_ext_write_all_unchecked_writes_until_range_is_complete() {
    let mut output =
        ScriptedOutput::new(vec![WriteStep::Accept(2), WriteStep::Accept(2)]);
    let input = [10, 20, 30, 40, 50];

    // SAFETY: `input[1..5]` is a valid source range.
    unsafe {
        output
            .write_all_unchecked(&input, 1, 4)
            .expect("write_all_unchecked should finish after partial writes");
    }

    assert_eq!(&[20, 30, 40, 50], output.values.as_slice());
}

#[test]
fn test_output_ext_write_all_unchecked_retries_interrupted_writes() {
    let mut output =
        ScriptedOutput::new(vec![WriteStep::Interrupted, WriteStep::Accept(3)]);

    // SAFETY: The full input range is valid.
    unsafe {
        output
            .write_all_unchecked(&[1, 2, 3], 0, 3)
            .expect("interrupted writes should be retried");
    }

    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_output_ext_write_all_unchecked_returns_write_zero() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Zero]);

    // SAFETY: The full input range is valid.
    let error = unsafe {
        output
            .write_all_unchecked(&[1, 2, 3], 0, 3)
            .expect_err("zero progress should fail")
    };

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_output_ext_write_all_writes_full_slice() {
    let mut output = ScriptedOutput::new(vec![WriteStep::Accept(3)]);

    output
        .write_all(&[7, 8, 9])
        .expect("full-slice write should succeed");

    assert_eq!(&[7, 8, 9], output.values.as_slice());
}
