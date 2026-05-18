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
    Write,
};

use qubit_io::TeeReader;

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("branch write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct InvalidCountReader;

impl Read for InvalidCountReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Ok(usize::MAX)
    }
}

#[test]
fn test_tee_reader_copies_read_bytes_to_branch_writer() {
    let source = Cursor::new(b"abcdef".to_vec());
    let branch = Vec::new();
    let mut reader = TeeReader::new(source, branch);
    assert_eq!(0, reader.reader_ref().position());
    assert!(reader.branch_ref().is_empty());

    let mut buffer = [0; 3];
    let count = reader.read(&mut buffer).expect("tee read should succeed");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
    assert_eq!(b"abc", reader.branch_ref().as_slice());

    let (source, branch) = reader.into_inner();
    assert_eq!(3, source.position());
    assert_eq!(b"abc", branch.as_slice());
}

#[test]
fn test_tee_reader_mut_accessors_allow_inner_access() {
    let source = Cursor::new(b"abc".to_vec());
    let branch = Vec::new();
    let mut reader = TeeReader::new(source, branch);

    reader.reader_mut().set_position(1);
    reader.branch_mut().extend_from_slice(b"x");
    let mut buffer = [0; 2];
    let count = reader.read(&mut buffer).expect("tee read should succeed");

    assert_eq!(2, count);
    assert_eq!(b"bc", &buffer);
    assert_eq!(b"xbc", reader.branch_ref().as_slice());
}

#[test]
fn test_tee_reader_returns_source_read_error() {
    let mut reader = TeeReader::new(FailingReader, Vec::new());
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("source read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert!(reader.branch_ref().is_empty());
}

#[test]
fn test_tee_reader_does_not_write_branch_at_eof() {
    let source = Cursor::new(Vec::new());
    let branch = Vec::new();
    let mut reader = TeeReader::new(source, branch);
    let mut buffer = [0; 2];

    let count = reader
        .read(&mut buffer)
        .expect("EOF should not be an error");

    assert_eq!(0, count);
    assert!(reader.branch_ref().is_empty());
}

#[test]
fn test_tee_reader_copies_partial_buffer_at_eof() {
    let source = Cursor::new(b"ab".to_vec());
    let branch = Vec::new();
    let mut reader = TeeReader::new(source, branch);
    let mut buffer = [0; 4];

    let count = reader
        .read(&mut buffer)
        .expect("partial read should succeed");

    assert_eq!(2, count);
    assert_eq!(b"ab", &buffer[..count]);
    assert_eq!(b"ab", reader.branch_ref().as_slice());
}

#[test]
#[should_panic]
fn test_tee_reader_panics_when_source_returns_invalid_count() {
    let mut reader = TeeReader::new(InvalidCountReader, Vec::new());
    let mut buffer = [0; 2];

    let _ = reader.read(&mut buffer);
}

#[test]
fn test_tee_reader_returns_branch_write_error() {
    let source = Cursor::new(b"abc".to_vec());
    let mut reader = TeeReader::new(source, FailingWriter);
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("branch write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch write failed", error.to_string());
}
