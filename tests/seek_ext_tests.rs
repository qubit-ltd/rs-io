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
    Seek,
    SeekFrom,
};

use qubit_io::SeekExt;

struct FailingSeek {
    original_position: u64,
    end_result: std::io::Result<u64>,
    restore_result: std::io::Result<u64>,
}

impl FailingSeek {
    fn length_error(original_position: u64) -> Self {
        Self {
            original_position,
            end_result: Err(Error::other("length failed")),
            restore_result: Ok(original_position),
        }
    }

    fn restore_error(original_position: u64, length: u64) -> Self {
        Self {
            original_position,
            end_result: Ok(length),
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
            SeekFrom::Current(_) | SeekFrom::End(_) => {
                Err(Error::new(ErrorKind::Unsupported, "unsupported seek"))
            }
        }
    }
}

struct PositionFailingSeek;

impl Seek for PositionFailingSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Err(Error::other("position failed")),
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => {
                Err(Error::new(ErrorKind::Unsupported, "unsupported seek"))
            }
        }
    }
}

#[test]
fn test_stream_len_preserving_position_returns_length_without_moving_cursor() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");

    let length = cursor
        .stream_len_preserving_position()
        .expect("stream length should be readable");

    assert_eq!(6, length);
    assert_eq!(
        2,
        cursor
            .stream_position()
            .expect("cursor position should be readable"),
    );
}

#[test]
fn test_stream_len_preserving_position_returns_length_error_after_restore() {
    let mut stream = FailingSeek::length_error(4);

    let error = stream
        .stream_len_preserving_position()
        .expect_err("length errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("length failed", error.to_string());
}

#[test]
fn test_stream_len_preserving_position_returns_restore_error() {
    let mut stream = FailingSeek::restore_error(4, 10);

    let error = stream
        .stream_len_preserving_position()
        .expect_err("restore errors should be reported");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_stream_len_preserving_position_returns_position_error() {
    let mut stream = PositionFailingSeek;

    let error = stream
        .stream_len_preserving_position()
        .expect_err("position errors should be returned immediately");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}

#[test]
fn test_stream_len_preserving_position_works_on_dyn_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(3))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn Seek = &mut cursor;

    let length = stream
        .stream_len_preserving_position()
        .expect("seek extension should work on dyn Seek");

    assert_eq!(6, length);
    assert_eq!(
        3,
        stream
            .stream_position()
            .expect("stream position should be restored"),
    );
}
