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
};

use qubit_io::PositionGuard;

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

struct RestoreFailingSeek {
    position: u64,
}

impl RestoreFailingSeek {
    fn new(position: u64) -> Self {
        Self { position }
    }
}

impl Seek for RestoreFailingSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.position),
            SeekFrom::Start(position) if position == self.position => {
                Err(Error::other("restore failed"))
            }
            SeekFrom::Start(position) => {
                self.position = position;
                Ok(position)
            }
            SeekFrom::Current(_) | SeekFrom::End(_) => {
                Err(Error::new(ErrorKind::Unsupported, "unsupported seek"))
            }
        }
    }
}

#[test]
fn test_position_guard_restores_on_drop() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(2))
        .expect("initial seek should succeed");

    {
        let mut guard = PositionGuard::new(&mut cursor)
            .expect("guard should capture position");
        assert_eq!(2, guard.position());
        guard
            .get_mut()
            .seek(SeekFrom::Start(5))
            .expect("seek through guard should succeed");
    }

    assert_eq!(
        2,
        cursor
            .stream_position()
            .expect("drop should restore original position")
    );
}

#[test]
fn test_position_guard_restore_restores_immediately() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(1))
        .expect("initial seek should succeed");
    let mut guard =
        PositionGuard::new(&mut cursor).expect("guard should capture position");

    guard
        .get_mut()
        .seek(SeekFrom::Start(4))
        .expect("seek through guard should succeed");
    guard.restore().expect("restore should succeed");

    assert_eq!(
        1,
        guard
            .get_mut()
            .stream_position()
            .expect("position should be restored")
    );
}

#[test]
fn test_position_guard_dismiss_skips_drop_restore() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    cursor
        .seek(SeekFrom::Start(1))
        .expect("initial seek should succeed");

    {
        let mut guard = PositionGuard::new(&mut cursor)
            .expect("guard should capture position");
        guard
            .get_mut()
            .seek(SeekFrom::Start(4))
            .expect("seek through guard should succeed");
        guard.dismiss();
    }

    assert_eq!(
        4,
        cursor
            .stream_position()
            .expect("dismissed guard should not restore")
    );
}

#[test]
fn test_position_guard_returns_position_error() {
    let mut stream = PositionFailingSeek;

    let error = match PositionGuard::new(&mut stream) {
        Ok(_) => panic!("position error should be returned"),
        Err(error) => error,
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("position failed", error.to_string());
}

#[test]
fn test_position_guard_restore_returns_restore_error() {
    let mut stream = RestoreFailingSeek::new(2);
    let mut guard =
        PositionGuard::new(&mut stream).expect("guard should capture position");

    let error = guard
        .restore()
        .expect_err("restore error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("restore failed", error.to_string());
}

#[test]
fn test_position_guard_drop_ignores_restore_error() {
    let mut stream = RestoreFailingSeek::new(2);

    {
        let _guard = PositionGuard::new(&mut stream)
            .expect("guard should capture position");
    }
}
