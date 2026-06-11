// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Write};

use qubit_io::LimitWriter;

struct PanicOnWrite;

impl Write for PanicOnWrite {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        panic!("exhausted limit must not write to inner writer")
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::other("flush failed"))
    }
}

#[test]
fn test_limit_writer_writes_at_most_limit() {
    let mut writer = LimitWriter::new(Vec::new(), 4);
    assert_eq!(4, writer.remaining());
    assert!(writer.get_ref().is_empty());

    let count = writer
        .write(b"abcdef")
        .expect("limited write should succeed");

    assert_eq!(4, count);
    assert_eq!(0, writer.remaining());

    let count = writer
        .write(b"z")
        .expect("exhausted writer should report zero bytes written");
    assert_eq!(0, count);

    let output = writer.into_inner();
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_limit_writer_get_mut_allows_inner_access() {
    let mut writer = LimitWriter::new(Vec::new(), 2);

    writer.get_mut().extend_from_slice(b"x");
    let count = writer.write(b"abc").expect("write should be limited");

    assert_eq!(2, count);
    assert_eq!(b"xab", writer.get_ref().as_slice());
}

#[test]
fn test_limit_writer_zero_limit_does_not_write_inner() {
    let mut writer = LimitWriter::new(PanicOnWrite, 0);

    let count = writer
        .write(b"a")
        .expect("zero limit should report zero bytes written");

    assert_eq!(0, count);
    assert_eq!(0, writer.remaining());
}

#[test]
fn test_limit_writer_empty_buffer_does_not_write_inner() {
    let mut writer = LimitWriter::new(PanicOnWrite, 1);

    let count = writer
        .write(b"")
        .expect("empty buffer should complete without writing");

    assert_eq!(0, count);
    assert_eq!(1, writer.remaining());
}

#[test]
fn test_limit_writer_preserves_remaining_on_error() {
    let mut writer = LimitWriter::new(FailingWriter, 3);

    let error = writer
        .write(b"ab")
        .expect_err("inner write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert_eq!(3, writer.remaining());
}

#[test]
fn test_limit_writer_flush_delegates_to_inner_writer() {
    let mut writer = LimitWriter::new(FailingWriter, 3);

    let error = writer
        .flush()
        .expect_err("inner flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
}
