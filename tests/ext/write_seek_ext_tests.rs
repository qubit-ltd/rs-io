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
    Seek,
    SeekFrom,
    Write,
};

use qubit_io::{
    WriteSeek,
    WriteSeekExt,
};

struct FailingWriteSeek {
    original_position: u64,
    rejected_offset: Option<u64>,
    fail_write: bool,
    fail_restore: bool,
}

impl FailingWriteSeek {
    fn write_error(original_position: u64) -> Self {
        Self {
            original_position,
            rejected_offset: None,
            fail_write: true,
            fail_restore: false,
        }
    }

    fn offset_seek_error(original_position: u64, rejected_offset: u64) -> Self {
        Self {
            original_position,
            rejected_offset: Some(rejected_offset),
            fail_write: false,
            fail_restore: false,
        }
    }

    fn write_and_restore_error(original_position: u64) -> Self {
        Self {
            original_position,
            rejected_offset: None,
            fail_write: true,
            fail_restore: true,
        }
    }

    fn offset_seek_and_restore_error(
        original_position: u64,
        rejected_offset: u64,
    ) -> Self {
        Self {
            original_position,
            rejected_offset: Some(rejected_offset),
            fail_write: false,
            fail_restore: true,
        }
    }

    fn restore_error(original_position: u64) -> Self {
        Self {
            original_position,
            rejected_offset: None,
            fail_write: false,
            fail_restore: true,
        }
    }
}

impl Write for FailingWriteSeek {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.fail_write {
            Err(Error::other("write failed"))
        } else {
            Ok(buffer.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for FailingWriteSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.original_position),
            SeekFrom::Start(position)
                if Some(position) == self.rejected_offset =>
            {
                Err(Error::other("offset seek failed"))
            }
            SeekFrom::Start(position)
                if position == self.original_position && self.fail_restore =>
            {
                Err(Error::other("restore failed"))
            }
            SeekFrom::Start(position) => Ok(position),
            SeekFrom::Current(_) | SeekFrom::End(_) => {
                Err(Error::new(ErrorKind::Unsupported, "unsupported seek"))
            }
        }
    }
}

struct PositionFailingWriteSeek;

impl Write for PositionFailingWriteSeek {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for PositionFailingWriteSeek {
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
fn test_write_all_at_preserving_position_writes_without_moving_cursor() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("cursor should seek to initial position");

    cursor
        .write_all_at_preserving_position(1, b"ZZ")
        .expect("position-preserving write should succeed");

    assert_eq!(b"aZZdef", cursor.get_ref().as_slice());
    assert_eq!(
        2,
        cursor
            .stream_position()
            .expect("cursor position should be restored"),
    );
}

#[test]
fn test_write_all_at_preserving_position_works_on_dyn_write_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(4))
        .expect("cursor should seek to initial position");
    let stream: &mut dyn WriteSeek = &mut cursor;

    stream
        .write_all_at_preserving_position(2, b"YY")
        .expect("write-seek extension should work on dyn WriteSeek");

    assert_eq!(
        4,
        stream
            .stream_position()
            .expect("stream position should be restored"),
    );
    assert_eq!(b"abYYef", cursor.get_ref().as_slice());
}

#[test]
fn test_write_all_at_preserving_position_ufcs_works_on_dyn_write_seek() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(4))
        .expect("cursor should seek to initial position");
    {
        let stream: &mut dyn WriteSeek = &mut cursor;
        <dyn WriteSeek as WriteSeekExt>::write_all_at_preserving_position(
            stream, 2, b"YY",
        )
        .expect("UFCS write-seek extension should work on dyn WriteSeek");
        assert_eq!(
            4,
            stream
                .stream_position()
                .expect("stream position should be restored"),
        );
    }

    assert_eq!(b"abYYef", cursor.get_ref().as_slice());
}

#[test]
fn test_write_all_at_preserving_position_returns_write_error_after_restore() {
    let mut writer = FailingWriteSeek::write_error(4);

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err("write errors should be returned after restoring position");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_write_all_at_preserving_position_prefers_restore_error_over_write_error()
 {
    let mut writer = FailingWriteSeek::write_and_restore_error(4);

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err("restore errors should take precedence over write errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_write_all_at_preserving_position_returns_offset_seek_error_after_restore()
 {
    let mut writer = FailingWriteSeek::offset_seek_error(4, 1);

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err(
            "offset seek errors should be returned after restoring position",
        );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("offset seek failed", error.to_string());
}

#[test]
fn test_write_all_at_preserving_position_prefers_restore_error_over_offset_seek_error()
 {
    let mut writer = FailingWriteSeek::offset_seek_and_restore_error(4, 1);

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err(
            "restore errors should take precedence over offset seek errors",
        );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_write_all_at_preserving_position_returns_restore_error() {
    let mut writer = FailingWriteSeek::restore_error(4);

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err("restore errors should be reported");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_write_all_at_preserving_position_returns_position_error() {
    let mut writer = PositionFailingWriteSeek;

    let error = writer
        .write_all_at_preserving_position(1, b"abc")
        .expect_err("position errors should be returned immediately");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}
