/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
};

use qubit_io::ReadExt;

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
fn test_read_exact_array_reads_array() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let array = reader
        .read_exact_array::<6>()
        .expect("array should be filled across short reads");

    assert_eq!(*b"abcdef", array);
}

#[test]
fn test_read_exact_array_zero_length_does_not_read() {
    let mut reader = PanicOnRead;

    let array = reader
        .read_exact_array::<0>()
        .expect("zero-length arrays should complete immediately");

    assert!(array.is_empty());
}

#[test]
fn test_read_exact_array_returns_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());

    let error = reader
        .read_exact_array::<3>()
        .expect_err("short input should return the standard read_exact EOF error");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_exact_vec_limited_reads_exact_length() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let data = reader
        .read_exact_vec_limited(6, 8)
        .expect("exact vector should be filled across short reads");

    assert_eq!(b"abcdef", data.as_slice());
}

#[test]
fn test_read_exact_vec_limited_zero_length_does_not_read() {
    let mut reader = PanicOnRead;

    let data = reader
        .read_exact_vec_limited(0, 0)
        .expect("zero-length exact reads should complete immediately");

    assert!(data.is_empty());
}

#[test]
fn test_read_exact_vec_limited_rejects_len_over_max_before_reading() {
    let mut reader = PanicOnRead;

    let error = reader
        .read_exact_vec_limited(4, 3)
        .expect_err("requested length over max should be rejected before reading");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "requested length 4 exceeds maximum length 3",
        error.to_string()
    );
}

#[test]
fn test_read_exact_vec_limited_reports_allocation_failure_before_reading() {
    let mut reader = PanicOnRead;

    let error = reader
        .read_exact_vec_limited(usize::MAX, usize::MAX)
        .expect_err("allocation failure should be returned before reading");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_read_exact_vec_limited_returns_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());

    let error = reader
        .read_exact_vec_limited(3, 3)
        .expect_err("short input should return the standard read_exact EOF error");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_exact_vec_limited_returns_read_error() {
    let mut reader = FailingReader;

    let error = reader
        .read_exact_vec_limited(3, 3)
        .expect_err("non-EOF read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_exact_vec_limited_into_appends_exact_length() {
    let mut reader = ShortReader::new(b"abcdef", 2);
    let mut output = b"prefix-".to_vec();

    reader
        .read_exact_vec_limited_into(&mut output, 6, 8)
        .expect("exact vector should be appended across short reads");

    assert_eq!(b"prefix-abcdef", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_zero_length_does_not_read() {
    let mut reader = PanicOnRead;
    let mut output = b"prefix".to_vec();

    reader
        .read_exact_vec_limited_into(&mut output, 0, 0)
        .expect("zero-length exact reads should leave output unchanged");

    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_rejects_len_over_max_without_changing_output() {
    let mut reader = PanicOnRead;
    let mut output = b"prefix".to_vec();

    let error = reader
        .read_exact_vec_limited_into(&mut output, 4, 3)
        .expect_err("requested length over max should be rejected before reading");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "requested length 4 exceeds maximum length 3",
        error.to_string()
    );
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_rolls_back_on_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());
    let mut output = b"prefix".to_vec();

    let error = reader
        .read_exact_vec_limited_into(&mut output, 3, 3)
        .expect_err("short input should return the standard read_exact EOF error");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_exact_vec_limited_into_rolls_back_on_read_error() {
    let mut reader = PartialThenFailReader::new(b"ab");
    let mut output = b"prefix".to_vec();

    let error = reader
        .read_exact_vec_limited_into(&mut output, 3, 3)
        .expect_err("read errors after partial append should roll back output");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed after prefix", error.to_string());
    assert_eq!(b"prefix", output.as_slice());
    assert_eq!(2, reader.position);
}

#[test]
fn test_read_exact_helpers_work_on_dyn_read_with_ufcs() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let array = <dyn Read as ReadExt>::read_exact_array::<2>(reader)
        .expect("UFCS read_exact_array should work on dyn Read");
    assert_eq!(*b"ab", array);

    let data = <dyn Read as ReadExt>::read_exact_vec_limited(reader, 2, 4)
        .expect("UFCS read_exact_vec_limited should work on dyn Read");
    assert_eq!(b"cd", data.as_slice());

    let mut output = b"prefix-".to_vec();
    <dyn Read as ReadExt>::read_exact_vec_limited_into(reader, &mut output, 2, 4)
        .expect("UFCS read_exact_vec_limited_into should work on dyn Read");
    assert_eq!(b"prefix-ef", output.as_slice());
}

#[test]
fn test_read_exact_or_eof_reads_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);
    let mut buffer = [0; 6];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("short reads should be retried until the buffer is full");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", &buffer);
}

#[test]
fn test_read_exact_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 2);
    let mut buffer = [b'x'; 5];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("EOF after partial data should not be an error");

    assert_eq!(3, count);
    assert_eq!(b"abcxx", &buffer);
}

#[test]
fn test_read_exact_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");
    let mut buffer = [0; 3];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("interrupted reads should be retried");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_exact_or_eof_empty_buffer_does_not_read() {
    let mut reader = PanicOnRead;
    let mut buffer = [];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("empty buffers should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_read_exact_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut buffer = [0; 3];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("read extension should work on dyn Read");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_ext_ufcs_methods_work_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut buffer = [0; 2];
    let count = <dyn Read as ReadExt>::read_exact_or_eof(reader, &mut buffer)
        .expect("UFCS read_exact_or_eof should work on dyn Read");
    assert_eq!(2, count);
    assert_eq!(b"ab", &buffer);

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let count = <dyn Read as ReadExt>::discard_exact_or_eof(reader, 2)
        .expect("UFCS discard_exact_or_eof should work on dyn Read");
    assert_eq!(2, count);

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count = <dyn Read as ReadExt>::copy_to(reader, &mut output)
        .expect("UFCS copy_to should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!(b"abc", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count = <dyn Read as ReadExt>::copy_to_at_most(reader, &mut output, 2)
        .expect("UFCS copy_to_at_most should work on dyn Read");
    assert_eq!(2, count);
    assert_eq!(b"ab", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count = <dyn Read as ReadExt>::copy_to_end_limited(reader, &mut output, 3)
        .expect("UFCS copy_to_end_limited should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!(b"abc", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let data = <dyn Read as ReadExt>::read_to_end_limited(reader, 3)
        .expect("UFCS read_to_end_limited should work on dyn Read");
    assert_eq!(b"abc", data.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = b"prefix-".to_vec();
    let count = <dyn Read as ReadExt>::read_to_end_limited_into(reader, &mut output, 3)
        .expect("UFCS read_to_end_limited_into should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!(b"prefix-abc", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let text = <dyn Read as ReadExt>::read_to_string_limited(reader, 3)
        .expect("UFCS read_to_string_limited should work on dyn Read");
    assert_eq!("abc", text);

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = String::from("prefix-");
    let count = <dyn Read as ReadExt>::read_to_string_limited_into(reader, &mut output, 3)
        .expect("UFCS read_to_string_limited_into should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!("prefix-abc", output);
}

#[test]
fn test_discard_exact_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let count = reader
        .discard_exact_or_eof(4)
        .expect("discard extension should work on dyn Read");

    assert_eq!(4, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"ef", remaining.as_slice());
}

#[test]
fn test_copy_to_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to(&mut output)
        .expect("copy extension should work on dyn Read");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", output.as_slice());
}

#[test]
fn test_copy_to_at_most_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to_at_most(&mut output, 4)
        .expect("limited copy extension should work on dyn Read");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_copy_to_end_limited_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcd".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to_end_limited(&mut output, 4)
        .expect("end-limited copy extension should work on dyn Read");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_read_exact_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut buffer = [0; 3];

    let error = reader
        .read_exact_or_eof(&mut buffer)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_discard_exact_or_eof_discards_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let count = reader
        .discard_exact_or_eof(5)
        .expect("short reads should be retried while discarding");

    assert_eq!(5, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"f", remaining.as_slice());
}

#[test]
fn test_discard_exact_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 1);

    let count = reader
        .discard_exact_or_eof(5)
        .expect("EOF after partial discard should not be an error");

    assert_eq!(3, count);
}

#[test]
fn test_discard_exact_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let count = reader
        .discard_exact_or_eof(3)
        .expect("interrupted reads should be retried while discarding");

    assert_eq!(3, count);
}

#[test]
fn test_discard_exact_or_eof_zero_bytes_does_not_read() {
    let mut reader = PanicOnRead;

    let count = reader
        .discard_exact_or_eof(0)
        .expect("zero-byte discard should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_discard_exact_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader
        .discard_exact_or_eof(3)
        .expect_err("non-interrupted read errors should be returned while discarding");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_end_limited_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let data = reader
        .read_to_end_limited(3)
        .expect("bounded read should work on dyn Read");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_end_limited_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let data = reader
        .read_to_end_limited(3)
        .expect("interrupted reads should be retried");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_end_limited_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader
        .read_to_end_limited(3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_end_limited_into_appends_to_existing_vec() {
    let mut reader = Cursor::new(b"abc".to_vec());
    let mut output = b"prefix-".to_vec();

    let count = reader
        .read_to_end_limited_into(&mut output, 3)
        .expect("input within the limit should be appended");

    assert_eq!(3, count);
    assert_eq!(b"prefix-abc", output.as_slice());
}

#[test]
fn test_read_to_end_limited_into_rejects_oversized_input_after_prefix() {
    let mut reader = Cursor::new(b"abcd".to_vec());
    let mut output = b"prefix-".to_vec();

    let error = reader
        .read_to_end_limited_into(&mut output, 3)
        .expect_err("oversized input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
    assert_eq!(b"prefix-abc", output.as_slice());
    assert_eq!(4, reader.position());
}

#[test]
fn test_read_to_end_limited_into_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut output = b"prefix".to_vec();

    let error = reader
        .read_to_end_limited_into(&mut output, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_to_end_limited_zero_limit_rejects_non_empty_input() {
    let mut reader = Cursor::new(b"a".to_vec());

    let error = reader
        .read_to_end_limited(0)
        .expect_err("zero limit should reject non-empty input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_to_end_limited_into_zero_limit_rejects_non_empty_input() {
    let mut reader = Cursor::new(b"a".to_vec());
    let mut output = b"prefix".to_vec();

    let error = reader
        .read_to_end_limited_into(&mut output, 0)
        .expect_err("zero limit should reject non-empty input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix", output.as_slice());
}

#[test]
fn test_read_to_end_limited_zero_limit_accepts_empty_input() {
    let mut reader = Cursor::new(Vec::new());

    let value = reader
        .read_to_end_limited(0)
        .expect("zero limit should accept empty input");

    assert!(value.is_empty());
}

#[test]
fn test_read_to_string_limited_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader
        .read_to_string_limited(3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_string_limited_reads_utf8_input() {
    let mut reader = Cursor::new("hello 世界".as_bytes().to_vec());

    let value = reader
        .read_to_string_limited(16)
        .expect("UTF-8 input within the limit should be read");

    assert_eq!("hello 世界", value);
}

#[test]
fn test_read_to_string_limited_into_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut output = String::from("prefix");

    let error = reader
        .read_to_string_limited_into(&mut output, 3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!("prefix", output);
}

#[test]
fn test_read_to_string_limited_into_appends_utf8_input() {
    let mut reader = Cursor::new("hello 世界".as_bytes().to_vec());
    let mut output = String::from("prefix-");

    let count = reader
        .read_to_string_limited_into(&mut output, 16)
        .expect("UTF-8 input within the limit should be appended");

    assert_eq!("hello 世界".len(), count);
    assert_eq!("prefix-hello 世界", output);
}

#[test]
fn test_read_to_string_limited_zero_limit_accepts_empty_input() {
    let mut reader = Cursor::new(Vec::new());

    let value = reader
        .read_to_string_limited(0)
        .expect("zero limit should accept empty UTF-8 input");

    assert!(value.is_empty());
}

#[test]
fn test_read_to_string_limited_rejects_oversized_input() {
    let mut reader = Cursor::new(b"abcd".to_vec());

    let error = reader
        .read_to_string_limited(3)
        .expect_err("oversized string input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("input exceeds maximum length of 3 bytes", error.to_string());
}

#[test]
fn test_read_to_string_limited_into_rejects_oversized_input_without_appending() {
    let mut reader = Cursor::new(b"abcd".to_vec());
    let mut output = String::from("prefix");

    let error = reader
        .read_to_string_limited_into(&mut output, 3)
        .expect_err("oversized string input should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix", output);
    assert_eq!(4, reader.position());
}

#[test]
fn test_read_to_string_limited_rejects_invalid_utf8() {
    let mut reader = Cursor::new(vec![0xff]);

    let error = reader
        .read_to_string_limited(4)
        .expect_err("invalid UTF-8 should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("limited input is not valid UTF-8")
    );
}

#[test]
fn test_read_to_string_limited_into_rejects_invalid_utf8_without_appending() {
    let mut reader = Cursor::new(vec![0xff]);
    let mut output = String::from("prefix");

    let error = reader
        .read_to_string_limited_into(&mut output, 4)
        .expect_err("invalid UTF-8 should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("limited input is not valid UTF-8")
    );
    assert_eq!("prefix", output);
}
