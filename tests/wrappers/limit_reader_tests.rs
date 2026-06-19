// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
};

use qubit_io::LimitReader;

struct PanicOnRead;

impl Read for PanicOnRead {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("exhausted limit must not read from inner reader")
    }
}

#[test]
fn test_limit_reader_forwards_buf_read_with_limit() {
    use std::io::{
        BufRead,
        BufReader,
        Cursor,
    };

    let inner = BufReader::new(Cursor::new(b"abcdef".to_vec()));
    let mut reader = qubit_io::LimitReader::new(inner, 3);

    assert_eq!(b"abc", reader.fill_buf().expect("buffer should be capped"));
    reader.consume(2);
    assert_eq!(1, reader.remaining());
    assert_eq!(
        b"c",
        reader.fill_buf().expect("remaining buffer should shrink")
    );
    reader.consume(1);
    assert_eq!(0, reader.remaining());
    assert!(
        reader
            .fill_buf()
            .expect("limit should be exhausted")
            .is_empty()
    );
}

#[test]
#[should_panic(expected = "cannot consume beyond limit reader")]
fn test_limit_reader_buf_read_consume_panics_when_count_exceeds_limit() {
    use std::io::{
        BufRead,
        BufReader,
        Cursor,
    };

    let inner = BufReader::new(Cursor::new(b"abcdef".to_vec()));
    let mut reader = qubit_io::LimitReader::new(inner, 3);

    assert_eq!(b"abc", reader.fill_buf().expect("buffer should be capped"));
    reader.consume(4);
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

#[test]
fn test_limit_reader_reads_at_most_limit() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut reader = LimitReader::new(cursor, 4);
    assert_eq!(4, reader.remaining());
    assert_eq!(0, reader.inner().position());

    let mut buffer = [0; 8];
    let count = reader
        .read(&mut buffer)
        .expect("limited read should succeed");

    assert_eq!(4, count);
    assert_eq!(b"abcd", &buffer[..count]);
    assert_eq!(0, reader.remaining());

    let count = reader
        .read(&mut buffer)
        .expect("exhausted reader should report EOF");
    assert_eq!(0, count);

    let inner = reader.into_inner();
    assert_eq!(4, inner.position());
}

#[test]
fn test_limit_reader_get_mut_allows_inner_access() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut reader = LimitReader::new(cursor, 2);

    reader.inner_mut().set_position(1);
    let mut buffer = [0; 4];
    let count = reader
        .read(&mut buffer)
        .expect("limited read should use mutated inner reader");

    assert_eq!(2, count);
    assert_eq!(b"bc", &buffer[..count]);
}

#[test]
fn test_limit_reader_zero_limit_does_not_read_inner() {
    let mut reader = LimitReader::new(PanicOnRead, 0);
    let mut buffer = [0; 1];

    let count = reader
        .read(&mut buffer)
        .expect("zero limit should behave like EOF");

    assert_eq!(0, count);
    assert_eq!(0, reader.remaining());
}

#[test]
fn test_limit_reader_empty_buffer_does_not_read_inner() {
    let mut reader = LimitReader::new(PanicOnRead, 1);
    let mut buffer = [];

    let count = reader
        .read(&mut buffer)
        .expect("empty buffer should complete without reading");

    assert_eq!(0, count);
    assert_eq!(1, reader.remaining());
}

#[test]
fn test_limit_reader_preserves_remaining_on_error() {
    let mut reader = LimitReader::new(FailingReader, 3);
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("inner read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(3, reader.remaining());
}
