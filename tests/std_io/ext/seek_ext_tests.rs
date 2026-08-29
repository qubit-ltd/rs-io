// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Seek;
use std::io::SeekFrom;

use qubit_io::std_io::ext::SeekExt;

struct FailingSeek {
    original_position: u64,
    end_result: std::io::Result<u64>,
    restore_result: std::io::Result<u64>,
}

impl FailingSeek {
    fn size_error(original_position: u64) -> Self {
        Self {
            original_position,
            end_result: Err(Error::other("size failed")),
            restore_result: Ok(original_position),
        }
    }

    fn restore_error(original_position: u64, size: u64) -> Self {
        Self {
            original_position,
            end_result: Ok(size),
            restore_result: Err(Error::other("restore failed")),
        }
    }

    fn size_and_restore_error(original_position: u64) -> Self {
        Self {
            original_position,
            end_result: Err(Error::other("size failed")),
            restore_result: Err(Error::other("restore failed")),
        }
    }
}

impl Seek for FailingSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.original_position),
            SeekFrom::End(0) => self
                .end_result
                .as_ref()
                .copied()
                .map_err(|error| Error::new(error.kind(), error.to_string())),
            SeekFrom::Start(position) if position == self.original_position => self
                .restore_result
                .as_ref()
                .copied()
                .map_err(|error| Error::new(error.kind(), error.to_string())),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

struct PositionFailingSeek;

impl Seek for PositionFailingSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Err(Error::other("position failed")),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

#[test]
fn test_stream_size_returns_size_without_moving_cursor() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");

    let size = cursor.stream_size().expect("stream size should be readable");

    assert_eq!(6, size);
    assert_eq!(2, cursor.stream_position().expect("cursor position should be readable"),);
}

#[test]
fn test_stream_size_returns_size_error_after_restore() {
    let mut stream = FailingSeek::size_error(4);

    let error = stream
        .stream_size()
        .expect_err("size errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("size failed", error.to_string());
}

#[test]
fn test_stream_size_returns_restore_error() {
    let mut stream = FailingSeek::restore_error(4, 10);

    let error = stream.stream_size().expect_err("restore errors should be reported");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_stream_size_prefers_restore_error_over_size_error() {
    let mut stream = FailingSeek::size_and_restore_error(4);

    let error = stream
        .stream_size()
        .expect_err("restore errors should take precedence over size errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_stream_size_returns_position_error() {
    let mut stream = PositionFailingSeek;

    let error = stream
        .stream_size()
        .expect_err("position errors should be returned immediately");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}

#[test]
fn test_stream_size_works_on_dyn_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(3))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn Seek = &mut cursor;

    let size = stream.stream_size().expect("seek extension should work on dyn Seek");

    assert_eq!(6, size);
    assert_eq!(3, stream.stream_position().expect("stream position should be restored"),);
}

#[test]
fn test_stream_size_ufcs_works_on_dyn_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn Seek = &mut cursor;

    let size = <dyn Seek as SeekExt>::stream_size(stream).expect("UFCS stream_size should work on dyn Seek");

    assert_eq!(6, size);
    assert_eq!(2, stream.stream_position().expect("stream position should be restored"),);
}
