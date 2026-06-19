// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0
// =============================================================================
use std::cmp::Ordering;
use std::io::{Cursor, Error, ErrorKind, Read, Write};

use qubit_io::{ReadExt, Streams};

struct InterruptedOnceReader {
    interrupted: bool,
    data: Cursor<Vec<u8>>,
}

impl InterruptedOnceReader {
    fn new(data: &[u8]) -> Self {
        Self {
            interrupted: false,
            data: Cursor::new(data.to_vec()),
        }
    }
}

impl Read for InterruptedOnceReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        self.data.read(buffer)
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct PanicOnRead;

impl Read for PanicOnRead {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("zero-byte copy must not read")
    }
}

struct InterruptThenEofReader {
    data: Cursor<Vec<u8>>,
    interrupted: bool,
}

impl InterruptThenEofReader {
    fn new(data: &[u8]) -> Self {
        Self {
            data: Cursor::new(data.to_vec()),
            interrupted: false,
        }
    }
}

impl Read for InterruptThenEofReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.data.position() < self.data.get_ref().len() as u64 {
            return self.data.read(buffer);
        }
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted at eof"));
        }
        Ok(0)
    }
}

struct FailAfterDataReader {
    data: Cursor<Vec<u8>>,
}

impl FailAfterDataReader {
    fn new(data: &[u8]) -> Self {
        Self {
            data: Cursor::new(data.to_vec()),
        }
    }
}

impl Read for FailAfterDataReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.data.position() < self.data.get_ref().len() as u64 {
            return self.data.read(buffer);
        }
        Err(Error::other("tail read failed"))
    }
}

#[test]
fn test_copy_at_most_copies_at_most_requested_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = Streams::copy_at_most(&mut input, &mut output, 4).expect("copy should succeed");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
}

#[test]
fn test_copy_at_most_returns_partial_count_at_eof() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = Vec::new();

    let copied =
        Streams::copy_at_most(&mut input, &mut output, 5).expect("copy should stop at EOF");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_copy_at_most_zero_bytes_does_not_read() {
    let mut input = PanicOnRead;
    let mut output = Vec::new();

    let copied =
        Streams::copy_at_most(&mut input, &mut output, 0).expect("zero-byte copy should succeed");

    assert_eq!(0, copied);
    assert!(output.is_empty());
}

#[test]
fn test_copy_at_most_retries_interrupted_reads() {
    let mut input = InterruptedOnceReader::new(b"abc");
    let mut output = Vec::new();

    let copied = Streams::copy_at_most(&mut input, &mut output, 3)
        .expect("interrupted reads should be retried");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_copy_at_most_returns_read_error() {
    let mut input = FailingReader;
    let mut output = Vec::new();

    let error = Streams::copy_at_most(&mut input, &mut output, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_copy_at_most_returns_write_error() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = FailingWriter;

    let error = Streams::copy_at_most(&mut input, &mut output, 3)
        .expect_err("write errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_copy_at_most_with_buffer_size_copies_at_most_requested_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = Streams::copy_at_most_with_buffer_size(&mut input, &mut output, 4, 1)
        .expect("copy should succeed with a caller-selected buffer");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
}

#[test]
fn test_copy_at_most_with_buffer_size_rejects_zero_size() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = Vec::new();

    let error = Streams::copy_at_most_with_buffer_size(&mut input, &mut output, 3, 0)
        .expect_err("zero-sized copy buffers should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        "copy buffer size must be greater than zero",
        error.to_string()
    );
}

#[test]
fn test_copy_copies_until_eof() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = Streams::copy(&mut input, &mut output).expect("copy should reach EOF");

    assert_eq!(6, copied);
    assert_eq!(b"abcdef", output.as_slice());
}

#[test]
fn test_copy_returns_read_error() {
    let mut input = FailingReader;
    let mut output = Vec::new();

    let error = Streams::copy(&mut input, &mut output)
        .expect_err("std copy read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_copy_functions_work_on_dyn_read_write() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut input;
    let mut output = Vec::new();
    let writer: &mut dyn Write = &mut output;

    let copied = Streams::copy_at_most::<dyn Read, dyn Write>(reader, writer, 3)
        .expect("dyn copy should succeed");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());

    let mut input = Cursor::new(b"xyz".to_vec());
    let reader: &mut dyn Read = &mut input;
    let mut output = Vec::new();
    let writer: &mut dyn Write = &mut output;

    let copied = Streams::copy_to_end_limited::<dyn Read, dyn Write>(reader, writer, 3)
        .expect("dyn end-limited copy should succeed");

    assert_eq!(3, copied);
    assert_eq!(b"xyz", output.as_slice());
}

#[test]
fn test_copy_to_method_copies_remaining_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = input
        .copy_to(&mut output)
        .expect("copy_to should copy until EOF");

    assert_eq!(6, copied);
    assert_eq!(b"abcdef", output.as_slice());
}

#[test]
fn test_copy_to_end_limited_returns_dyn_tail_probe_error() {
    let mut input = FailingReader;
    let reader: &mut dyn Read = &mut input;
    let mut output = Vec::new();

    let error = Streams::copy_to_end_limited::<dyn Read, Vec<u8>>(reader, &mut output, 0)
        .expect_err("dyn tail probe errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_copy_to_at_most_method_copies_at_most_requested_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = input
        .copy_to_at_most(&mut output, 4)
        .expect("copy_to_at_most should stop at the limit");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
}

#[test]
fn test_copy_to_end_limited_copies_exact_length() {
    let mut input = Cursor::new(b"abcd".to_vec());
    let mut output = Vec::new();

    let copied = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect("copy_to_end_limited should accept exact-length input");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
}

#[test]
fn test_copy_to_end_limited_copies_shorter_input() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = Vec::new();

    let copied = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect("copy_to_end_limited should stop at EOF");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
    assert_eq!(3, input.position());
}

#[test]
fn test_copy_to_end_limited_rejects_oversized_input() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let error = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect_err("copy_to_end_limited should reject oversized input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 4 bytes", error.to_string());
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(5, input.position());
}

#[test]
fn test_copy_to_end_limited_retries_interrupted_tail_probe() {
    let mut input = InterruptThenEofReader::new(b"abcd");
    let mut output = Vec::new();

    let copied = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect("interrupted EOF probe should be retried");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_copy_to_end_limited_returns_copy_read_error() {
    let mut input = FailingReader;
    let mut output = Vec::new();

    let error = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect_err("copy read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert!(output.is_empty());
}

#[test]
fn test_copy_to_end_limited_returns_copy_write_error() {
    let mut input = Cursor::new(b"abcd".to_vec());
    let mut output = FailingWriter;

    let error = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect_err("copy write errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_copy_to_end_limited_returns_tail_probe_error() {
    let mut input = FailAfterDataReader::new(b"abcd");
    let mut output = Vec::new();

    let error = Streams::copy_to_end_limited(&mut input, &mut output, 4)
        .expect_err("tail probe read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("tail read failed", error.to_string());
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_copy_to_end_limited_method_rejects_oversized_input() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let error = input
        .copy_to_end_limited(&mut output, 4)
        .expect_err("copy_to_end_limited method should reject oversized input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(5, input.position());
}

#[test]
fn test_read_to_end_limited_returns_vec_when_input_fits() {
    let mut input = Cursor::new(b"abc".to_vec());

    let data = input
        .read_to_end_limited(3)
        .expect("input within limit should be read");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_end_limited_rejects_input_beyond_limit() {
    let mut input = Cursor::new(b"abcd".to_vec());

    let error = input
        .read_to_end_limited(3)
        .expect_err("input beyond limit should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_content_eq_compares_streams() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut same = Cursor::new(b"abc".to_vec());

    assert!(Streams::content_eq(&mut left, &mut same).expect("equal streams should compare"));

    let mut left = Cursor::new(b"abc".to_vec());
    let mut different = Cursor::new(b"abd".to_vec());

    assert!(
        !Streams::content_eq(&mut left, &mut different).expect("different streams should compare")
    );
}

#[test]
fn test_content_eq_returns_read_error() {
    let mut left = FailingReader;
    let mut right = Cursor::new(b"abc".to_vec());

    let error = Streams::content_eq(&mut left, &mut right)
        .expect_err("content_eq should return compare read errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_compare_content_returns_lexicographic_ordering() {
    let mut less = Cursor::new(b"abc".to_vec());
    let mut greater = Cursor::new(b"abd".to_vec());
    let mut prefix = Cursor::new(b"ab".to_vec());
    let mut full = Cursor::new(b"abc".to_vec());

    assert_eq!(
        Ordering::Less,
        Streams::compare_content(&mut less, &mut greater).expect("streams should compare")
    );
    assert_eq!(
        Ordering::Less,
        Streams::compare_content(&mut prefix, &mut full).expect("prefix should compare")
    );

    let mut full = Cursor::new(b"abc".to_vec());
    let mut prefix = Cursor::new(b"ab".to_vec());
    assert_eq!(
        Ordering::Greater,
        Streams::compare_content(&mut full, &mut prefix).expect("full stream should compare")
    );

    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = Cursor::new(b"abc".to_vec());
    assert_eq!(
        Ordering::Equal,
        Streams::compare_content(&mut left, &mut right).expect("equal streams should compare")
    );
}

#[test]
fn test_compare_content_with_buffer_size_uses_requested_size() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = Cursor::new(b"abd".to_vec());

    assert_eq!(
        Ordering::Less,
        Streams::compare_content_with_buffer_size(&mut left, &mut right, 1)
            .expect("streams should compare with a caller-selected buffer")
    );
    assert_eq!(3, left.position());
    assert_eq!(3, right.position());
}

#[test]
fn test_compare_content_with_buffer_size_rejects_zero_size() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = Cursor::new(b"abc".to_vec());

    let error = Streams::compare_content_with_buffer_size(&mut left, &mut right, 0)
        .expect_err("zero-sized compare buffers should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        "compare buffer size must be greater than zero",
        error.to_string()
    );
}

#[test]
fn test_compare_content_returns_left_read_error() {
    let mut left = FailingReader;
    let mut right = Cursor::new(b"abc".to_vec());

    let error = Streams::compare_content(&mut left, &mut right)
        .expect_err("left read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_compare_content_returns_right_read_error() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = FailingReader;

    let error = Streams::compare_content(&mut left, &mut right)
        .expect_err("right read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_copy_at_most_impl_retries_interrupted_reads_for_typed_streams() {
    let mut input = InterruptedOnceReader::new(b"abc");
    let mut output = Vec::new();

    let copied = Streams::copy_at_most(&mut input, &mut output, 3)
        .expect("typed copy_at_most should retry interrupted reads");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_compare_content_with_buffer_size_continues_across_equal_chunks() {
    let mut left = Cursor::new(b"abcdef".to_vec());
    let mut right = Cursor::new(b"abcdef".to_vec());

    assert_eq!(
        Ordering::Equal,
        Streams::compare_content_with_buffer_size(&mut left, &mut right, 2)
            .expect("equal streams should compare across multiple chunks")
    );
    assert_eq!(6, left.position());
    assert_eq!(6, right.position());
}

#[test]
fn test_compare_content_with_buffer_size_detects_shorter_stream() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = Cursor::new(b"abcd".to_vec());

    assert_eq!(
        Ordering::Less,
        Streams::compare_content_with_buffer_size(&mut left, &mut right, 4)
            .expect("shorter stream should compare as less")
    );
}

#[test]
fn test_compare_content_with_buffer_size_returns_allocation_error() {
    let mut left = Cursor::new(b"a".to_vec());
    let mut right = Cursor::new(b"a".to_vec());

    let error = Streams::compare_content_with_buffer_size(&mut left, &mut right, usize::MAX)
        .expect_err("absurd compare buffers should fail allocation");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_copy_at_most_with_buffer_size_returns_allocation_error() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = Vec::new();

    let error = Streams::copy_at_most_with_buffer_size(&mut input, &mut output, 3, usize::MAX)
        .expect_err("absurd copy buffers should fail allocation");

    assert_eq!(ErrorKind::Other, error.kind());
}
