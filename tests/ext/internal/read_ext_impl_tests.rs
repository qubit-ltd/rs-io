// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, Error, ErrorKind, Read};

use qubit_io::ext::internal::read_ext_impl;

struct ShortReader {
    data: Vec<u8>,
    position: usize,
    max_chunk: usize,
}

impl ShortReader {
    fn new(data: &[u8], max_chunk: usize) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
            max_chunk,
        }
    }
}

impl Read for ShortReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.data.len() {
            return Ok(0);
        }
        let remaining = self.data.len() - self.position;
        let count = remaining.min(buffer.len()).min(self.max_chunk);
        buffer[..count].copy_from_slice(&self.data[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

struct InterruptedOnceReader {
    interrupted: bool,
    inner: ShortReader,
}

impl InterruptedOnceReader {
    fn new(data: &[u8]) -> Self {
        Self {
            interrupted: false,
            inner: ShortReader::new(data, data.len().max(1)),
        }
    }
}

impl Read for InterruptedOnceReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        self.inner.read(buffer)
    }
}

struct PanicOnRead;

impl Read for PanicOnRead {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("empty operations must not call read")
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct PartialThenFailReader {
    data: Vec<u8>,
    position: usize,
}

impl PartialThenFailReader {
    fn new(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
        }
    }
}

impl Read for PartialThenFailReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.position < self.data.len() {
            let count = (self.data.len() - self.position).min(buffer.len());
            buffer[..count].copy_from_slice(&self.data[self.position..self.position + count]);
            self.position += count;
            return Ok(count);
        }
        Err(Error::other("read failed after prefix"))
    }
}

#[test]
fn test_validate_exact_read_len_accepts_len_within_max() {
    read_ext_impl::validate_exact_read_len(3, 4).expect("len within max should succeed");
}

#[test]
fn test_validate_exact_read_len_rejects_len_over_max() {
    let error = read_ext_impl::validate_exact_read_len(4, 3)
        .expect_err("requested length over max should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "requested length 4 exceeds maximum length 3",
        error.to_string()
    );
}

#[test]
fn test_read_exact_or_eof_reads_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);
    let mut buffer = [0; 6];

    let count = read_ext_impl::read_exact_or_eof(&mut reader, &mut buffer)
        .expect("short reads should be retried until the buffer is full");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", &buffer);
}

#[test]
fn test_read_exact_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 2);
    let mut buffer = [b'x'; 5];

    let count = read_ext_impl::read_exact_or_eof(&mut reader, &mut buffer)
        .expect("EOF after partial data should not be an error");

    assert_eq!(3, count);
    assert_eq!(b"abcxx", &buffer);
}

#[test]
fn test_read_exact_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");
    let mut buffer = [0; 3];

    let count = read_ext_impl::read_exact_or_eof(&mut reader, &mut buffer)
        .expect("interrupted reads should be retried");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_exact_or_eof_empty_buffer_does_not_read() {
    let mut reader = PanicOnRead;
    let mut buffer = [];

    let count = read_ext_impl::read_exact_or_eof(&mut reader, &mut buffer)
        .expect("empty buffers should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_read_exact_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut buffer = [0; 3];

    let count = read_ext_impl::read_exact_or_eof(reader, &mut buffer)
        .expect("read_ext_impl::read_exact_or_eof should work on dyn Read");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_exact_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut buffer = [0; 3];

    let error = read_ext_impl::read_exact_or_eof(&mut reader, &mut buffer)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_exact_vec_limited_into_appends_exact_length() {
    let mut reader = ShortReader::new(b"abcdef", 2);
    let mut output = b"prefix-".to_vec();

    read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 6, 8)
        .expect("exact vector should be appended across short reads");

    assert_eq!(b"prefix-abcdef", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_zero_length_does_not_read() {
    let mut reader = PanicOnRead;
    let mut output = b"prefix".to_vec();

    read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 0, 0)
        .expect("zero-length exact reads should leave output unchanged");

    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_rejects_len_over_max_without_changing_output() {
    let mut reader = PanicOnRead;
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 4, 3)
        .expect_err("requested length over max should be rejected before reading");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "requested length 4 exceeds maximum length 3",
        error.to_string()
    );
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_reports_length_overflow_before_reading() {
    let mut reader = PanicOnRead;
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, usize::MAX, usize::MAX)
        .expect_err("overflowing output length should fail before reading");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        format!("length 6 plus {} overflows usize", usize::MAX),
        error.to_string()
    );
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_reports_allocation_failure_before_reading() {
    let mut reader = PanicOnRead;
    let mut output = Vec::new();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, usize::MAX, usize::MAX)
        .expect_err("allocation failure should be returned before reading");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_read_exact_vec_limited_into_returns_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());
    let mut output = Vec::new();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 3, 3)
        .expect_err("short input should return the standard read_exact EOF error");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_exact_vec_limited_into_returns_read_error() {
    let mut reader = FailingReader;
    let mut output = Vec::new();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 3, 3)
        .expect_err("non-EOF read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_exact_vec_limited_into_rolls_back_on_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 3, 3)
        .expect_err("short input should return the standard read_exact EOF error");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_rolls_back_on_read_error() {
    let mut reader = PartialThenFailReader::new(b"ab");
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_exact_vec_limited_into(&mut reader, &mut output, 3, 3)
        .expect_err("read errors after partial append should roll back output");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed after prefix", error.to_string());
    assert_eq!(b"prefix", output.as_slice());
    assert_eq!(2, reader.position);
}

#[test]
fn test_read_exact_vec_limited_into_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdefgh".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = b"prefix-".to_vec();

    read_ext_impl::read_exact_vec_limited_into(reader, &mut output, 2, 4)
        .expect("read_ext_impl::read_exact_vec_limited_into should work on dyn Read");

    assert_eq!(b"prefix-ab", output.as_slice());
}

#[test]
fn test_read_to_end_limited_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let data = read_ext_impl::read_to_end_limited(reader, 3).expect("bounded read should work on dyn Read");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_end_limited_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let data = read_ext_impl::read_to_end_limited(&mut reader, 3)
        .expect("interrupted reads should be retried");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_end_limited_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = read_ext_impl::read_to_end_limited(&mut reader, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_end_limited_into_appends_to_existing_vec() {
    let mut reader = Cursor::new(b"abc".to_vec());
    let mut output = b"prefix-".to_vec();

    let count = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect("input within the limit should be appended");

    assert_eq!(3, count);
    assert_eq!(b"prefix-abc", output.as_slice());
}

#[test]
fn test_read_to_end_limited_rejects_oversized_input_on_first_read() {
    let mut reader = Cursor::new(b"abcd".to_vec());

    let error = read_ext_impl::read_to_end_limited(&mut reader, 3)
        .expect_err("oversized input should be rejected on the first read");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert_eq!(4, reader.position());
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_on_first_read() {
    let mut reader = Cursor::new(b"abcd".to_vec());
    let mut output = Vec::new();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("oversized input should be rejected on the first read");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert!(output.is_empty(), "output must be rolled back to its original length");
    assert_eq!(4, reader.position());
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_after_prefix() {
    let mut reader = Cursor::new(b"abcd".to_vec());
    let mut output = b"prefix-".to_vec();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("oversized input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert_eq!(b"prefix-", output.as_slice());
    assert_eq!(4, reader.position());
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_after_partial_reads() {
    let mut reader = ShortReader::new(b"abcd", 1);
    let mut output = Vec::new();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("oversized input should be rejected after partial reads");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert!(
        output.is_empty(),
        "partially appended bytes must be rolled back on overflow"
    );
    assert_eq!(4, reader.position);
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_when_last_read_exceeds_remaining(
) {
    let mut reader = ShortReader::new(b"abcX", 2);
    let mut output = Vec::new();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("a read returning more than remaining quota should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert!(
        output.is_empty(),
        "bytes appended before the overflow read must be rolled back"
    );
    assert_eq!(4, reader.position);
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_after_prefix_and_partial_reads() {
    let mut reader = ShortReader::new(b"abcde", 1);
    let mut output = b"prefix-".to_vec();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("oversized input should be rejected after prefix and partial reads");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert_eq!(
        b"prefix-",
        output.as_slice(),
        "prefix and partially appended bytes must be rolled back on overflow"
    );
    assert_eq!(4, reader.position);
}

#[test]
fn test_read_to_end_limited_into_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_to_end_limited_zero_limit_rejects_non_empty_input() {
    let mut reader = Cursor::new(b"a".to_vec());

    let error = read_ext_impl::read_to_end_limited(&mut reader, 0)
        .expect_err("zero limit should reject non-empty input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_to_end_limited_into_zero_limit_rejects_non_empty_input() {
    let mut reader = Cursor::new(b"a".to_vec());
    let mut output = b"prefix".to_vec();

    let error = read_ext_impl::read_to_end_limited_into(&mut reader, &mut output, 0)
        .expect_err("zero limit should reject non-empty input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_to_end_limited_zero_limit_accepts_empty_input() {
    let mut reader = Cursor::new(Vec::new());

    let value = read_ext_impl::read_to_end_limited(&mut reader, 0)
        .expect("zero limit should accept empty input");

    assert!(value.is_empty());
}

#[test]
fn test_invalid_utf8_error_wraps_conversion_error() {
    let utf8_error = String::from_utf8(vec![0xff]).unwrap_err();
    let error = read_ext_impl::invalid_utf8_error(utf8_error);

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("limited input is not valid UTF-8")
    );
}

#[test]
fn test_read_to_end_limited_reads_utf8_input() {
    let mut reader = Cursor::new("hello 世界".as_bytes().to_vec());

    let bytes = read_ext_impl::read_to_end_limited(&mut reader, 16)
        .expect("UTF-8 input within the limit should be read");

    let value = String::from_utf8(bytes).expect("collected bytes should be valid UTF-8");
    assert_eq!("hello 世界", value);
}

#[test]
fn test_read_to_end_limited_rejects_oversized_utf8_input() {
    let mut reader = Cursor::new(b"abcd".to_vec());

    let error = read_ext_impl::read_to_end_limited(&mut reader, 3)
        .expect_err("oversized string input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
}

#[test]
fn test_read_to_end_limited_zero_limit_accepts_empty_utf8_input() {
    let mut reader = Cursor::new(Vec::new());

    let bytes = read_ext_impl::read_to_end_limited(&mut reader, 0)
        .expect("zero limit should accept empty UTF-8 input");

    assert!(bytes.is_empty());
}

#[test]
fn test_read_to_end_limited_collects_invalid_utf8_bytes() {
    let mut reader = Cursor::new(vec![0xff]);

    let bytes = read_ext_impl::read_to_end_limited(&mut reader, 4)
        .expect("invalid UTF-8 bytes should still be collected");

    let error = String::from_utf8(bytes).expect_err("collected bytes should not be valid UTF-8");
    let io_error = read_ext_impl::invalid_utf8_error(error);

    assert_eq!(ErrorKind::InvalidData, io_error.kind());
    assert!(
        io_error
            .to_string()
            .starts_with("limited input is not valid UTF-8")
    );
}
