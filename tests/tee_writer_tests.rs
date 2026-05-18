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
    Error,
    ErrorKind,
    Write,
};

use qubit_io::TeeWriter;

struct ShortWriter {
    data: Vec<u8>,
    max_chunk: usize,
}

impl ShortWriter {
    fn new(max_chunk: usize) -> Self {
        Self {
            data: Vec::new(),
            max_chunk,
        }
    }
}

impl Write for ShortWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let count = buffer.len().min(self.max_chunk);
        self.data.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FailingWriter {
    message: &'static str,
}

impl FailingWriter {
    fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other(self.message))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::other(self.message))
    }
}

struct InvalidCountWriter;

impl Write for InvalidCountWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        Ok(buffer.len() + 1)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_tee_writer_copies_written_bytes_to_branch_writer() {
    let primary = ShortWriter::new(3);
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);
    assert_eq!(0, writer.primary_ref().data.len());
    assert!(writer.branch_ref().is_empty());

    let count = writer.write(b"abcdef").expect("tee write should succeed");

    assert_eq!(3, count);
    assert_eq!(b"abc", writer.primary_ref().data.as_slice());
    assert_eq!(b"abc", writer.branch_ref().as_slice());

    let (primary, branch) = writer.into_inner();
    assert_eq!(b"abc", primary.data.as_slice());
    assert_eq!(b"abc", branch.as_slice());
}

#[test]
fn test_tee_writer_mut_accessors_allow_inner_access() {
    let primary = ShortWriter::new(4);
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    writer.primary_mut().data.extend_from_slice(b"x");
    writer.branch_mut().extend_from_slice(b"y");
    let count = writer.write(b"ab").expect("tee write should succeed");

    assert_eq!(2, count);
    assert_eq!(b"xab", writer.primary_ref().data.as_slice());
    assert_eq!(b"yab", writer.branch_ref().as_slice());
}

#[test]
fn test_tee_writer_allows_primary_zero_length_write() {
    let primary = ShortWriter::new(0);
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    let count = writer
        .write(b"ab")
        .expect("zero-length primary write should succeed");

    assert_eq!(0, count);
    assert!(writer.primary_ref().data.is_empty());
    assert!(writer.branch_ref().is_empty());
}

#[test]
fn test_tee_writer_returns_primary_write_error() {
    let primary = FailingWriter::new("primary write failed");
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .write(b"ab")
        .expect_err("primary write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("primary write failed", error.to_string());
    assert!(writer.branch_ref().is_empty());
}

#[test]
#[should_panic]
fn test_tee_writer_panics_when_primary_returns_invalid_count() {
    let primary = InvalidCountWriter;
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    let _ = writer.write(b"ab");
}

#[test]
fn test_tee_writer_returns_branch_write_error() {
    let primary = ShortWriter::new(4);
    let branch = FailingWriter::new("branch write failed");
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .write(b"ab")
        .expect_err("branch write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch write failed", error.to_string());
    assert_eq!(b"ab", writer.primary_ref().data.as_slice());
}

#[test]
fn test_tee_writer_flushes_both_writers() {
    let primary = Vec::new();
    let branch = FailingWriter::new("branch flush failed");
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .flush()
        .expect_err("branch flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch flush failed", error.to_string());
}

#[test]
fn test_tee_writer_flush_succeeds_when_both_writers_flush() {
    let primary = Vec::new();
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    writer.flush().expect("flush should succeed");
}

#[test]
fn test_tee_writer_returns_primary_flush_error() {
    let primary = FailingWriter::new("primary flush failed");
    let branch = Vec::new();
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .flush()
        .expect_err("primary flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("primary flush failed", error.to_string());
    assert!(writer.branch_ref().is_empty());
}
