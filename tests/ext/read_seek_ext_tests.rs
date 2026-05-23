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
    Seek,
    SeekFrom,
};

use qubit_io::{
    ReadSeek,
    ReadSeekExt,
};

struct RestoreFailingReader {
    restored_position: u64,
    read_once: bool,
}

impl RestoreFailingReader {
    fn new(restored_position: u64) -> Self {
        Self {
            restored_position,
            read_once: false,
        }
    }
}

impl Read for RestoreFailingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.read_once || buffer.is_empty() {
            return Ok(0);
        }
        self.read_once = true;
        buffer[0] = b'x';
        Ok(1)
    }
}

impl Seek for RestoreFailingReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.restored_position),
            SeekFrom::Start(position) if position == self.restored_position => Err(Error::other("restore failed")),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

struct ReadFailingReadSeek {
    original_position: u64,
    fail_restore: bool,
}

impl ReadFailingReadSeek {
    fn new(original_position: u64) -> Self {
        Self {
            original_position,
            fail_restore: false,
        }
    }

    fn with_restore_error(original_position: u64) -> Self {
        Self {
            original_position,
            fail_restore: true,
        }
    }
}

impl Read for ReadFailingReadSeek {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

impl Seek for ReadFailingReadSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.original_position),
            SeekFrom::Start(position) if position == self.original_position && self.fail_restore => {
                Err(Error::other("restore failed"))
            }
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

struct OffsetSeekFailingReadSeek {
    original_position: u64,
    rejected_offset: u64,
    fail_restore: bool,
}

impl OffsetSeekFailingReadSeek {
    fn new(original_position: u64, rejected_offset: u64) -> Self {
        Self {
            original_position,
            rejected_offset,
            fail_restore: false,
        }
    }

    fn with_restore_error(original_position: u64, rejected_offset: u64) -> Self {
        Self {
            original_position,
            rejected_offset,
            fail_restore: true,
        }
    }
}

impl Read for OffsetSeekFailingReadSeek {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Seek for OffsetSeekFailingReadSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.original_position),
            SeekFrom::Start(position) if position == self.original_position && self.fail_restore => {
                Err(Error::other("restore failed"))
            }
            SeekFrom::Start(position) if position == self.rejected_offset => Err(Error::other("offset seek failed")),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

struct PositionFailingReadSeek;

impl Read for PositionFailingReadSeek {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Seek for PositionFailingReadSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Err(Error::other("position failed")),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

struct InterruptedReadSeek {
    original_position: u64,
    interrupted: bool,
    read_once: bool,
}

impl InterruptedReadSeek {
    fn new(original_position: u64) -> Self {
        Self {
            original_position,
            interrupted: false,
            read_once: false,
        }
    }
}

impl Read for InterruptedReadSeek {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        if self.read_once || buffer.is_empty() {
            return Ok(0);
        }
        self.read_once = true;
        buffer[0] = b'x';
        Ok(1)
    }
}

impl Seek for InterruptedReadSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.original_position),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

#[test]
fn test_peek_exact_or_eof_reads_prefix_without_moving_cursor() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");
    let mut buffer = [0; 3];

    let count = cursor
        .peek_exact_or_eof(&mut buffer)
        .expect("peek should read from the current position");

    assert_eq!(3, count);
    assert_eq!(b"cde", &buffer);
    assert_eq!(2, cursor.stream_position().expect("cursor position should be readable"),);
}

#[test]
fn test_peek_exact_or_eof_returns_partial_count_at_eof() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    cursor
        .seek(SeekFrom::Start(1))
        .expect("cursor should seek to initial position");
    let mut buffer = [b'x'; 4];

    let count = cursor
        .peek_exact_or_eof(&mut buffer)
        .expect("peek should treat EOF as a partial result");

    assert_eq!(2, count);
    assert_eq!(b"bcxx", &buffer);
    assert_eq!(1, cursor.stream_position().expect("cursor position should be restored"),);
}

#[test]
fn test_peek_exact_or_eof_returns_restore_error_when_restore_fails() {
    let mut reader = RestoreFailingReader::new(7);
    let mut buffer = [0; 1];

    let error = reader
        .peek_exact_or_eof(&mut buffer)
        .expect_err("restore failures should be reported");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_peek_exact_or_eof_returns_read_error_after_restore() {
    let mut reader = ReadFailingReadSeek::new(3);
    let mut buffer = [0; 1];

    let error = reader
        .peek_exact_or_eof(&mut buffer)
        .expect_err("read errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_peek_exact_or_eof_prefers_restore_error_over_read_error() {
    let mut reader = ReadFailingReadSeek::with_restore_error(3);
    let mut buffer = [0; 1];

    let error = reader
        .peek_exact_or_eof(&mut buffer)
        .expect_err("restore errors should take precedence over read errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_peek_exact_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedReadSeek::new(3);
    let mut buffer = [0; 1];

    let count = reader
        .peek_exact_or_eof(&mut buffer)
        .expect("interrupted reads should be retried");

    assert_eq!(1, count);
    assert_eq!(b"x", &buffer);
}

#[test]
fn test_peek_exact_or_eof_returns_position_error() {
    let mut reader = PositionFailingReadSeek;
    let mut buffer = [0; 1];

    let error = reader
        .peek_exact_or_eof(&mut buffer)
        .expect_err("position errors should be returned immediately");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_reads_from_offset_without_moving_cursor() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");
    let mut buffer = [0; 4];

    let count = cursor
        .read_exact_or_eof_at(1, &mut buffer)
        .expect("offset read should read from the requested position");

    assert_eq!(4, count);
    assert_eq!(b"bcde", &buffer);
    assert_eq!(2, cursor.stream_position().expect("cursor position should be restored"),);
}

#[test]
fn test_read_exact_or_eof_at_works_on_dyn_read_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(4))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn ReadSeek = &mut cursor;
    let mut buffer = [0; 3];

    let count = stream
        .read_exact_or_eof_at(1, &mut buffer)
        .expect("offset read extension should work on dyn ReadSeek");

    assert_eq!(3, count);
    assert_eq!(b"bcd", &buffer);
    assert_eq!(4, stream.stream_position().expect("stream position should be restored"),);
}

#[test]
fn test_read_seek_ext_ufcs_methods_work_on_dyn_read_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(4))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn ReadSeek = &mut cursor;

    let mut peek_buffer = [0; 2];
    let count = <dyn ReadSeek as ReadSeekExt>::peek_exact_or_eof(stream, &mut peek_buffer)
        .expect("UFCS peek extension should work on dyn ReadSeek");
    assert_eq!(2, count);
    assert_eq!(b"ef", &peek_buffer);
    assert_eq!(4, stream.stream_position().expect("stream position should be restored"));

    let mut offset_buffer = [0; 3];
    let count = <dyn ReadSeek as ReadSeekExt>::read_exact_or_eof_at(stream, 1, &mut offset_buffer)
        .expect("UFCS offset read extension should work on dyn ReadSeek");
    assert_eq!(3, count);
    assert_eq!(b"bcd", &offset_buffer);
    assert_eq!(4, stream.stream_position().expect("stream position should be restored"));
}

#[test]
fn test_read_exact_or_eof_at_returns_offset_seek_error_after_restore() {
    let mut reader = OffsetSeekFailingReadSeek::new(5, 2);
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(2, &mut buffer)
        .expect_err("offset seek errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("offset seek failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_prefers_restore_error_over_offset_seek_error() {
    let mut reader = OffsetSeekFailingReadSeek::with_restore_error(5, 2);
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(2, &mut buffer)
        .expect_err("restore errors should take precedence over offset seek errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_returns_read_error_after_restore() {
    let mut reader = ReadFailingReadSeek::new(3);
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(1, &mut buffer)
        .expect_err("read errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_prefers_restore_error_over_read_error() {
    let mut reader = ReadFailingReadSeek::with_restore_error(3);
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(1, &mut buffer)
        .expect_err("restore errors should take precedence over read errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_returns_restore_error_when_restore_fails() {
    let mut reader = RestoreFailingReader::new(7);
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(1, &mut buffer)
        .expect_err("restore failures should be reported");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_read_exact_or_eof_at_returns_position_error() {
    let mut reader = PositionFailingReadSeek;
    let mut buffer = [0; 1];

    let error = reader
        .read_exact_or_eof_at(1, &mut buffer)
        .expect_err("position errors should be returned immediately");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}

#[test]
fn test_peek_exact_or_eof_works_on_dyn_read_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(1))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn ReadSeek = &mut cursor;
    let mut buffer = [0; 2];

    let count = stream
        .peek_exact_or_eof(&mut buffer)
        .expect("read-seek extension should work on dyn ReadSeek");

    assert_eq!(2, count);
    assert_eq!(b"bc", &buffer);
    assert_eq!(1, stream.stream_position().expect("stream position should be restored"),);
}
