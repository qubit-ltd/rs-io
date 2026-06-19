// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{Error, ErrorKind};

use qubit_io::{Input, InputExt, Output};

struct ChunkInput {
    chunks: VecDeque<Vec<u16>>,
}

impl ChunkInput {
    fn new(chunks: Vec<Vec<u16>>) -> Self {
        Self {
            chunks: VecDeque::from(chunks),
        }
    }
}

impl Input for ChunkInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        let Some(chunk) = self.chunks.pop_front() else {
            return Ok(0);
        };
        let read = count.min(chunk.len());
        output[index..index + read].copy_from_slice(&chunk[..read]);
        if read < chunk.len() {
            self.chunks.push_front(chunk[read..].to_vec());
        }
        Ok(read)
    }
}

#[derive(Default)]
struct CollectOutput {
    values: Vec<u16>,
}

impl Output for CollectOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        self.values.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct InterruptedInput {
    interrupted: bool,
    inner: ChunkInput,
}

impl InterruptedInput {
    fn new(chunks: Vec<Vec<u16>>) -> Self {
        Self {
            interrupted: false,
            inner: ChunkInput::new(chunks),
        }
    }
}

impl Input for InterruptedInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted"));
        }
        // SAFETY: The caller supplied the same valid destination range.
        unsafe { self.inner.read_unchecked(output, index, count) }
    }
}

struct OverreportingInput;

impl Input for OverreportingInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u16],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
    }
}

#[test]
fn test_input_ext_read_exact_or_eof_reads_until_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3]]);
    let mut output = [0_u16; 5];

    let read = input
        .read_exact_or_eof(&mut output)
        .expect("partial EOF should not be an error");

    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_returns_unexpected_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2]]);
    let mut output = [0_u16; 3];

    let error = input
        .read_exact(&mut output)
        .expect_err("short input should fail exact reads");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!([1, 2, 0], output);
}

#[test]
fn test_input_ext_read_exact_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3]]);
    let mut output = [0_u16; 3];

    input
        .read_exact(&mut output)
        .expect("interrupted reads should be retried");

    assert_eq!([1, 2, 3], output);
}

#[test]
fn test_input_ext_copy_to_at_most_copies_requested_items() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_at_most(&mut output, 3)
        .expect("bounded item copy should succeed");

    assert_eq!(3, copied);
    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_end_limited_rejects_oversized_input() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4]]);
    let mut output = CollectOutput::default();

    let error = input
        .copy_to_end_limited(&mut output, 3)
        .expect_err("oversized input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 items", error.to_string());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_end_limited_rejects_oversized_input_after_prefix() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3, 4]]);
    let mut output = CollectOutput { values: vec![9, 9] };

    let error = input
        .copy_to_end_limited(&mut output, 3)
        .expect_err("oversized input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(&[9, 9], output.values.as_slice());
}

#[test]
fn test_input_ext_read_exact_succeeds_when_buffer_is_filled() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
    let mut output = [0_u16; 3];

    input
        .read_exact(&mut output)
        .expect("exact read should succeed when enough items are available");

    assert_eq!([1, 2, 3], output);
}

#[test]
fn test_input_ext_read_exact_or_eof_rejects_overreported_count() {
    let mut input = OverreportingInput;
    let mut output = [0_u16; 2];

    let error = input
        .read_exact_or_eof(&mut output)
        .expect_err("overreported read count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_input_ext_read_exact_or_eof_unchecked_reads_into_middle_range() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let read = unsafe {
        input
            .read_exact_or_eof_unchecked(&mut output, 2, 4)
            .expect("middle range should be filled across short reads")
    };

    assert_eq!(4, read);
    assert_eq!([0, 0, 1, 2, 3, 4, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_or_eof_unchecked_returns_partial_count_at_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let read = unsafe {
        input
            .read_exact_or_eof_unchecked(&mut output, 2, 4)
            .expect("EOF after partial data should not be an error")
    };

    assert_eq!(3, read);
    assert_eq!([0, 0, 1, 2, 3, 0, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_or_eof_unchecked_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3, 4]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let read = unsafe {
        input
            .read_exact_or_eof_unchecked(&mut output, 2, 4)
            .expect("interrupted reads should be retried")
    };

    assert_eq!(4, read);
    assert_eq!([0, 0, 1, 2, 3, 4, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_or_eof_unchecked_returns_non_interrupted_error() {
    let mut input = FailingInput;
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let error = unsafe {
        input
            .read_exact_or_eof_unchecked(&mut output, 2, 4)
            .expect_err("non-interrupted read errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!([0, 0, 0, 0, 0, 0, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_unchecked_reads_into_middle_range() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    unsafe {
        input
            .read_exact_unchecked(&mut output, 2, 4)
            .expect("middle range should be filled across short reads");
    }

    assert_eq!([0, 0, 1, 2, 3, 4, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_unchecked_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3, 4]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    unsafe {
        input
            .read_exact_unchecked(&mut output, 2, 4)
            .expect("interrupted reads should be retried");
    }

    assert_eq!([0, 0, 1, 2, 3, 4, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_unchecked_returns_non_interrupted_error() {
    let mut input = FailingInput;
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let error = unsafe {
        input
            .read_exact_unchecked(&mut output, 2, 4)
            .expect_err("non-interrupted read errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!([0, 0, 0, 0, 0, 0, 0, 0], output);
}

#[test]
fn test_input_ext_read_exact_unchecked_returns_unexpected_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3]]);
    let mut output = [0_u16, 0, 0, 0, 0, 0, 0, 0];

    // SAFETY: `index..index + count` is `2..6`, which is within `output`.
    let error = unsafe {
        input
            .read_exact_unchecked(&mut output, 2, 4)
            .expect_err("short input should return unexpected EOF")
    };

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!([0, 0, 1, 2, 3, 0, 0, 0], output);
}

#[test]
fn test_input_ext_copy_to_copies_all_remaining_items() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to(&mut output)
        .expect("copy_to should copy until EOF");

    assert_eq!(4, copied);
    assert_eq!(&[1, 2, 3, 4], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to(&mut output)
        .expect("interrupted reads should be retried while copying");

    assert_eq!(3, copied);
    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_at_most_zero_returns_immediately() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_at_most(&mut output, 0)
        .expect("zero-byte copy should succeed");

    assert_eq!(0, copied);
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_end_limited_copies_within_limit() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_end_limited(&mut output, 4)
        .expect("input within limit should be copied");

    assert_eq!(3, copied);
    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_helpers_accept_generic_output_type() {
    let mut input = ChunkInput::new(vec![vec![1, 2]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to::<CollectOutput>(&mut output)
        .expect("generic output copy should succeed");
    assert_eq!(2, copied);
    assert_eq!(&[1, 2], output.values.as_slice());

    let mut input = ChunkInput::new(vec![vec![3, 4]]);
    let mut output = CollectOutput::default();
    let copied = input
        .copy_to_at_most::<CollectOutput>(&mut output, 1)
        .expect("generic bounded copy should succeed");
    assert_eq!(1, copied);
    assert_eq!(&[3], output.values.as_slice());

    let mut input = ChunkInput::new(vec![vec![5, 6]]);
    let mut output = CollectOutput::default();
    let copied = input
        .copy_to_end_limited::<CollectOutput>(&mut output, 2)
        .expect("generic limited-end copy should succeed");
    assert_eq!(2, copied);
    assert_eq!(&[5, 6], output.values.as_slice());
}

struct FailingInput;

impl Input for FailingInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u16],
        _index: usize,
        _count: usize,
    ) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct FailOnWriteOutput {
    values: Vec<u16>,
    fail: bool,
}

impl Output for FailOnWriteOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        if self.fail {
            return Err(Error::other("write failed"));
        }
        self.values.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_input_ext_read_exact_or_eof_returns_read_error() {
    let mut input = FailingInput;
    let mut output = [0_u16; 2];

    let error = input
        .read_exact_or_eof(&mut output)
        .expect_err("read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_input_ext_copy_to_returns_read_error() {
    let mut input = FailingInput;
    let mut output = CollectOutput::default();

    let error = input
        .copy_to(&mut output)
        .expect_err("copy read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_returns_write_error() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
    let mut output = FailOnWriteOutput {
        values: Vec::new(),
        fail: true,
    };

    let error = input
        .copy_to(&mut output)
        .expect_err("copy write errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_input_ext_copy_to_at_most_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_at_most(&mut output, 3)
        .expect("interrupted reads should be retried while copying");

    assert_eq!(3, copied);
    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_end_limited_retries_interrupted_reads() {
    let mut input = InterruptedInput::new(vec![vec![1, 2, 3]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_end_limited(&mut output, 3)
        .expect("interrupted reads should be retried while copying");

    assert_eq!(3, copied);
    assert_eq!(&[1, 2, 3], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_at_most_stops_at_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_at_most(&mut output, 10)
        .expect("copy_to_at_most should stop at EOF");

    assert_eq!(2, copied);
    assert_eq!(&[1, 2], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_rejects_overreported_read_count() {
    let mut input = OverreportingInput;
    let mut output = CollectOutput::default();

    let error = input
        .copy_to(&mut output)
        .expect_err("overreported read counts should fail during copy");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_end_limited_flushes_collected_items_at_eof() {
    let mut input = ChunkInput::new(vec![vec![1, 2], vec![3, 4, 5]]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_end_limited(&mut output, 10)
        .expect("copy_to_end_limited should flush collected items at EOF");

    assert_eq!(5, copied);
    assert_eq!(&[1, 2, 3, 4, 5], output.values.as_slice());
}

#[test]
fn test_input_ext_copy_to_end_limited_returns_write_error_when_flushing_collected() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
    let mut output = FailOnWriteOutput {
        values: Vec::new(),
        fail: true,
    };

    let error = input
        .copy_to_end_limited(&mut output, 3)
        .expect_err("flush of collected items should propagate write errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_read_exact_propagates_read_error() {
    let mut input = FailingInput;
    let mut output = [0_u16; 2];

    let error = input
        .read_exact(&mut output)
        .expect_err("read_exact should propagate read errors from read_exact_or_eof");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_input_ext_copy_to_at_most_propagates_read_error() {
    let mut input = FailingInput;
    let mut output = CollectOutput::default();

    let error = input
        .copy_to_at_most(&mut output, 3)
        .expect_err("copy_to_at_most should propagate read errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_end_limited_propagates_read_error() {
    let mut input = FailingInput;
    let mut output = CollectOutput::default();

    let error = input
        .copy_to_end_limited(&mut output, 3)
        .expect_err("copy_to_end_limited should propagate read errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_end_limited_returns_zero_for_empty_input() {
    let mut input = ChunkInput::new(vec![]);
    let mut output = CollectOutput::default();

    let copied = input
        .copy_to_end_limited(&mut output, 3)
        .expect("empty input should succeed without writing");

    assert_eq!(0, copied);
    assert!(output.values.is_empty());
}

#[test]
fn test_input_ext_copy_to_at_most_returns_write_error() {
    let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
    let mut output = FailOnWriteOutput {
        values: Vec::new(),
        fail: true,
    };

    let error = input
        .copy_to_at_most(&mut output, 3)
        .expect_err("copy_to_at_most should propagate write errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}
