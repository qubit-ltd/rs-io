/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0
 *
 ******************************************************************************/
use std::cmp::Ordering;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
    Write,
};

use qubit_io::{
    ReadExt,
    compare_content,
    content_eq,
    copy_limited,
};

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

#[test]
fn test_copy_limited_copies_at_most_requested_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = copy_limited(&mut input, &mut output, 4).expect("copy should succeed");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
}

#[test]
fn test_copy_limited_returns_partial_count_at_eof() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = Vec::new();

    let copied = copy_limited(&mut input, &mut output, 5).expect("copy should stop at EOF");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_copy_limited_zero_bytes_does_not_read() {
    let mut input = PanicOnRead;
    let mut output = Vec::new();

    let copied = copy_limited(&mut input, &mut output, 0).expect("zero-byte copy should succeed");

    assert_eq!(0, copied);
    assert!(output.is_empty());
}

#[test]
fn test_copy_limited_retries_interrupted_reads() {
    let mut input = InterruptedOnceReader::new(b"abc");
    let mut output = Vec::new();

    let copied =
        copy_limited(&mut input, &mut output, 3).expect("interrupted reads should be retried");

    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_copy_limited_returns_read_error() {
    let mut input = FailingReader;
    let mut output = Vec::new();

    let error = copy_limited(&mut input, &mut output, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_copy_limited_returns_write_error() {
    let mut input = Cursor::new(b"abc".to_vec());
    let mut output = FailingWriter;

    let error =
        copy_limited(&mut input, &mut output, 3).expect_err("write errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
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
fn test_copy_to_limited_method_copies_at_most_requested_bytes() {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();

    let copied = input
        .copy_to_limited(&mut output, 4)
        .expect("copy_to_limited should stop at the limit");

    assert_eq!(4, copied);
    assert_eq!(b"abcd", output.as_slice());
    assert_eq!(4, input.position());
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

    assert!(content_eq(&mut left, &mut same).expect("equal streams should compare"));

    let mut left = Cursor::new(b"abc".to_vec());
    let mut different = Cursor::new(b"abd".to_vec());

    assert!(!content_eq(&mut left, &mut different).expect("different streams should compare"));
}

#[test]
fn test_compare_content_returns_lexicographic_ordering() {
    let mut less = Cursor::new(b"abc".to_vec());
    let mut greater = Cursor::new(b"abd".to_vec());
    let mut prefix = Cursor::new(b"ab".to_vec());
    let mut full = Cursor::new(b"abc".to_vec());

    assert_eq!(
        Ordering::Less,
        compare_content(&mut less, &mut greater).expect("streams should compare")
    );
    assert_eq!(
        Ordering::Less,
        compare_content(&mut prefix, &mut full).expect("prefix should compare")
    );

    let mut full = Cursor::new(b"abc".to_vec());
    let mut prefix = Cursor::new(b"ab".to_vec());
    assert_eq!(
        Ordering::Greater,
        compare_content(&mut full, &mut prefix).expect("full stream should compare")
    );

    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = Cursor::new(b"abc".to_vec());
    assert_eq!(
        Ordering::Equal,
        compare_content(&mut left, &mut right).expect("equal streams should compare")
    );
}

#[test]
fn test_compare_content_returns_left_read_error() {
    let mut left = FailingReader;
    let mut right = Cursor::new(b"abc".to_vec());

    let error =
        compare_content(&mut left, &mut right).expect_err("left read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_compare_content_returns_right_read_error() {
    let mut left = Cursor::new(b"abc".to_vec());
    let mut right = FailingReader;

    let error =
        compare_content(&mut left, &mut right).expect_err("right read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}
