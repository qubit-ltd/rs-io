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

use qubit_io::CountingReader;

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

#[test]
fn test_counting_reader_counts_successful_reads() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut reader = CountingReader::new(cursor);
    assert_eq!(0, reader.bytes_read());
    assert_eq!(0, reader.get_ref().position());

    let mut buffer = [0; 4];
    let first = reader.read(&mut buffer).expect("first read should succeed");
    let second = reader
        .read(&mut buffer)
        .expect("second read should succeed");
    let eof = reader
        .read(&mut buffer)
        .expect("EOF should not be an error");

    assert_eq!(4, first);
    assert_eq!(2, second);
    assert_eq!(0, eof);
    assert_eq!(6, reader.bytes_read());

    let inner = reader.into_inner();
    assert_eq!(6, inner.position());
}

#[test]
fn test_counting_reader_get_mut_allows_inner_access() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut reader = CountingReader::new(cursor);

    reader.get_mut().set_position(1);
    let mut buffer = [0; 4];
    let count = reader
        .read(&mut buffer)
        .expect("read should use mutated inner reader");

    assert_eq!(2, count);
    assert_eq!(2, reader.bytes_read());
    assert_eq!(b"bc", &buffer[..count]);
}

#[test]
fn test_counting_reader_does_not_count_failed_reads() {
    let mut reader = CountingReader::new(FailingReader);
    let mut buffer = [0; 1];

    let error = reader
        .read(&mut buffer)
        .expect_err("inner read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(0, reader.bytes_read());
}
