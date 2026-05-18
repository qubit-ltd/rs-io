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

#[test]
fn test_read_fully_or_eof_reads_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);
    let mut buffer = [0; 6];

    let count = reader
        .read_fully_or_eof(&mut buffer)
        .expect("short reads should be retried until the buffer is full");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", &buffer);
}

#[test]
fn test_read_fully_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 2);
    let mut buffer = [b'x'; 5];

    let count = reader
        .read_fully_or_eof(&mut buffer)
        .expect("EOF after partial data should not be an error");

    assert_eq!(3, count);
    assert_eq!(b"abcxx", &buffer);
}

#[test]
fn test_read_fully_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");
    let mut buffer = [0; 3];

    let count = reader
        .read_fully_or_eof(&mut buffer)
        .expect("interrupted reads should be retried");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_fully_or_eof_empty_buffer_does_not_read() {
    let mut reader = PanicOnRead;
    let mut buffer = [];

    let count = reader
        .read_fully_or_eof(&mut buffer)
        .expect("empty buffers should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_read_fully_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut buffer = [0; 3];

    let count = reader
        .read_fully_or_eof(&mut buffer)
        .expect("read extension should work on dyn Read");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_discard_fully_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let count = reader
        .discard_fully_or_eof(4)
        .expect("discard extension should work on dyn Read");

    assert_eq!(4, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"ef", remaining.as_slice());
}

#[test]
fn test_read_fully_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut buffer = [0; 3];

    let error = reader
        .read_fully_or_eof(&mut buffer)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_discard_fully_or_eof_discards_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let count = reader
        .discard_fully_or_eof(5)
        .expect("short reads should be retried while discarding");

    assert_eq!(5, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"f", remaining.as_slice());
}

#[test]
fn test_discard_fully_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 1);

    let count = reader
        .discard_fully_or_eof(5)
        .expect("EOF after partial discard should not be an error");

    assert_eq!(3, count);
}

#[test]
fn test_discard_fully_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let count = reader
        .discard_fully_or_eof(3)
        .expect("interrupted reads should be retried while discarding");

    assert_eq!(3, count);
}

#[test]
fn test_discard_fully_or_eof_zero_bytes_does_not_read() {
    let mut reader = PanicOnRead;

    let count = reader
        .discard_fully_or_eof(0)
        .expect("zero-byte discard should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_discard_fully_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader
        .discard_fully_or_eof(3)
        .expect_err("non-interrupted read errors should be returned while discarding");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_vec_limited_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let data = reader
        .read_to_vec_limited(3)
        .expect("bounded read should work on dyn Read");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_vec_limited_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let data = reader
        .read_to_vec_limited(3)
        .expect("interrupted reads should be retried");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_vec_limited_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader
        .read_to_vec_limited(3)
        .expect_err("non-interrupted read errors should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_vec_limited_zero_limit_rejects_non_empty_input() {
    let mut reader = Cursor::new(b"a".to_vec());

    let error = reader
        .read_to_vec_limited(0)
        .expect_err("zero limit should reject non-empty input");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
