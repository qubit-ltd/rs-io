// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Write};

use qubit_io::WriteExt;

struct ShortWriter {
    output: Vec<u8>,
    max_chunk: usize,
}

impl ShortWriter {
    fn new(max_chunk: usize) -> Self {
        Self {
            output: Vec::new(),
            max_chunk,
        }
    }
}

impl Write for ShortWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let count = buffer.len().min(self.max_chunk);
        self.output.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct InterruptedOnceWriter {
    interrupted: bool,
    inner: ShortWriter,
}

impl InterruptedOnceWriter {
    fn new(max_chunk: usize) -> Self {
        Self {
            interrupted: false,
            inner: ShortWriter::new(max_chunk),
        }
    }
}

impl Write for InterruptedOnceWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct ZeroWriter;

impl Write for ZeroWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Ok(0)
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
        Ok(())
    }
}

#[test]
fn test_write_unchecked_writes_middle_range_once() {
    let mut writer = ShortWriter::new(2);
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    let count = unsafe {
        writer
            .write_unchecked(&buffer, 2, 4)
            .expect("middle range should accept a short write")
    };

    assert_eq!(2, count);
    assert_eq!(b"ab", writer.output.as_slice());
}

#[test]
fn test_write_unchecked_returns_write_error() {
    let mut writer = FailingWriter;
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    let error = unsafe {
        writer
            .write_unchecked(&buffer, 2, 4)
            .expect_err("write errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_write_all_unchecked_writes_middle_range() {
    let mut writer = ShortWriter::new(2);
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    unsafe {
        writer
            .write_all_unchecked(&buffer, 2, 4)
            .expect("middle range should be written");
    }

    assert_eq!(b"abcd", writer.output.as_slice());
}

#[test]
fn test_write_all_unchecked_retries_interrupted_writes() {
    let mut writer = InterruptedOnceWriter::new(2);
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    unsafe {
        writer
            .write_all_unchecked(&buffer, 2, 4)
            .expect("interrupted write should be retried");
    }

    assert_eq!(b"abcd", writer.inner.output.as_slice());
}

#[test]
fn test_write_all_unchecked_returns_non_interrupted_error() {
    let mut writer = FailingWriter;
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    let error = unsafe {
        writer
            .write_all_unchecked(&buffer, 2, 4)
            .expect_err("non-interrupted write errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_write_all_unchecked_returns_write_zero() {
    let mut writer = ZeroWriter;
    let buffer = *b"xxabcdyy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` for the duration of the write.
    let error = unsafe {
        writer
            .write_all_unchecked(&buffer, 2, 4)
            .expect_err("zero-length write should fail")
    };

    assert_eq!(ErrorKind::WriteZero, error.kind());
}
