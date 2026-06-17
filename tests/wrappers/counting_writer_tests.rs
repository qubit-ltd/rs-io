// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Write};

use qubit_io::CountingWriter;

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
fn test_counting_writer_counts_successful_writes() {
    let mut writer = CountingWriter::new(Vec::new());
    assert_eq!(0, writer.bytes_written());
    assert!(writer.get_ref().is_empty());

    let first = writer.write(b"abc").expect("first write should succeed");
    let second = writer.write(b"de").expect("second write should succeed");

    assert_eq!(3, first);
    assert_eq!(2, second);
    assert_eq!(5, writer.bytes_written());

    let output = writer.into_inner();
    assert_eq!(b"abcde", output.as_slice());
}

#[test]
fn test_counting_writer_get_mut_allows_inner_access() {
    let mut writer = CountingWriter::new(Vec::new());

    writer.get_mut().extend_from_slice(b"x");
    let count = writer.write(b"ab").expect("write should succeed");

    assert_eq!(2, count);
    assert_eq!(2, writer.bytes_written());
    assert_eq!(b"xab", writer.get_ref().as_slice());
}

#[test]
fn test_counting_writer_does_not_count_failed_writes() {
    let mut writer = CountingWriter::new(FailingWriter);

    let error = writer
        .write(b"ab")
        .expect_err("inner write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert_eq!(0, writer.bytes_written());
}

#[test]
fn test_counting_writer_flush_delegates_to_inner_writer() {
    let mut writer = CountingWriter::new(FailingWriter);

    let error = writer
        .flush()
        .expect_err("inner flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
}

#[test]
fn test_counting_writer_forwards_seek_without_counting() {
    use std::io::{Cursor, Seek, SeekFrom};

    let mut writer = CountingWriter::new(Cursor::new(Vec::new()));

    writer
        .write_all(b"abc")
        .expect("initial write should succeed");
    writer
        .seek(SeekFrom::Start(1))
        .expect("seek should be forwarded");
    writer.write_all(b"z").expect("patch write should succeed");

    assert_eq!(4, writer.bytes_written());
    assert_eq!(b"azc", writer.into_inner().into_inner().as_slice());
}
