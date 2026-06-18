// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Error,
    ErrorKind,
    Read,
    Seek,
    SeekFrom,
    Write,
};

use qubit_io::{
    SyncSeekTeeReader,
    TeeReader,
};

enum ReadAction {
    Bytes(Vec<u8>),
    Error(&'static str),
    InvalidCount,
}

struct ScriptedReader {
    action: ReadAction,
}

impl ScriptedReader {
    fn bytes(bytes: &[u8]) -> Self {
        Self {
            action: ReadAction::Bytes(bytes.to_vec()),
        }
    }

    fn error(message: &'static str) -> Self {
        Self {
            action: ReadAction::Error(message),
        }
    }

    fn invalid_count() -> Self {
        Self {
            action: ReadAction::InvalidCount,
        }
    }

    fn replace_bytes(&mut self, bytes: &[u8]) {
        self.action = ReadAction::Bytes(bytes.to_vec());
    }

    fn remaining_len(&self) -> usize {
        match &self.action {
            ReadAction::Bytes(bytes) => bytes.len(),
            ReadAction::Error(_) | ReadAction::InvalidCount => 0,
        }
    }
}

impl Read for ScriptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.action {
            ReadAction::Bytes(bytes) => {
                let count = buffer.len().min(bytes.len());
                buffer[..count].copy_from_slice(&bytes[..count]);
                bytes.drain(..count);
                Ok(count)
            }
            ReadAction::Error(message) => Err(Error::other(*message)),
            ReadAction::InvalidCount => Ok(usize::MAX),
        }
    }
}

struct ScriptedBranch {
    data: Vec<u8>,
    error: Option<&'static str>,
}

impl ScriptedBranch {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            error: None,
        }
    }

    fn failing(message: &'static str) -> Self {
        Self {
            data: Vec::new(),
            error: Some(message),
        }
    }

    fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }
}

impl Write for ScriptedBranch {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if let Some(message) = self.error {
            return Err(Error::other(message));
        }
        self.data.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct ScriptedSeek {
    position: u64,
    seek_calls: usize,
    error: Option<&'static str>,
}

impl ScriptedSeek {
    fn new(position: u64) -> Self {
        Self {
            position,
            seek_calls: 0,
            error: None,
        }
    }

    fn failing(position: u64, message: &'static str) -> Self {
        Self {
            position,
            seek_calls: 0,
            error: Some(message),
        }
    }
}

impl Seek for ScriptedSeek {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.seek_calls += 1;
        if let Some(message) = self.error {
            return Err(Error::other(message));
        }
        match position {
            SeekFrom::Start(position) => {
                self.position = position;
                Ok(self.position)
            }
            SeekFrom::Current(offset) => {
                let target = i128::from(self.position) + i128::from(offset);
                self.position = u64::try_from(target).map_err(|_| {
                    Error::new(ErrorKind::InvalidInput, "negative seek target")
                })?;
                Ok(self.position)
            }
            SeekFrom::End(_) => {
                Err(Error::new(ErrorKind::Unsupported, "unsupported seek"))
            }
        }
    }
}

#[test]
fn test_tee_reader_copies_read_bytes_to_branch_writer() {
    let source = ScriptedReader::bytes(b"abcdef");
    let branch = ScriptedBranch::new();
    let mut reader = TeeReader::new(source, branch);
    assert_eq!(6, reader.reader_ref().remaining_len());
    assert!(reader.branch_ref().as_slice().is_empty());

    let mut buffer = [0; 3];
    let count = reader.read(&mut buffer).expect("tee read should succeed");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
    assert_eq!(3, reader.reader_ref().remaining_len());
    assert_eq!(b"abc", reader.branch_ref().as_slice());

    let (source, branch) = reader.into_inner();
    assert_eq!(3, source.remaining_len());
    assert_eq!(b"abc", branch.as_slice());
}

#[test]
fn test_tee_reader_mut_accessors_allow_inner_access() {
    let source = ScriptedReader::bytes(b"abc");
    let branch = ScriptedBranch::new();
    let mut reader = TeeReader::new(source, branch);

    reader.reader_mut().replace_bytes(b"bc");
    reader.branch_mut().data.extend_from_slice(b"x");
    let mut buffer = [0; 2];
    let count = reader.read(&mut buffer).expect("tee read should succeed");

    assert_eq!(2, count);
    assert_eq!(b"bc", &buffer);
    assert_eq!(b"xbc", reader.branch_ref().as_slice());
}

#[test]
fn test_tee_reader_returns_source_read_error() {
    let mut reader = TeeReader::new(
        ScriptedReader::error("read failed"),
        ScriptedBranch::new(),
    );
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("source read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert!(reader.branch_ref().as_slice().is_empty());
}

#[test]
fn test_tee_reader_does_not_write_branch_at_eof() {
    let source = ScriptedReader::bytes(b"");
    let branch = ScriptedBranch::new();
    let mut reader = TeeReader::new(source, branch);
    let mut buffer = [0; 2];

    let count = reader
        .read(&mut buffer)
        .expect("EOF should not be an error");

    assert_eq!(0, count);
    assert!(reader.branch_ref().as_slice().is_empty());
}

#[test]
fn test_tee_reader_copies_partial_buffer_at_eof() {
    let source = ScriptedReader::bytes(b"ab");
    let branch = ScriptedBranch::new();
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
    let mut reader =
        TeeReader::new(ScriptedReader::invalid_count(), ScriptedBranch::new());
    let mut buffer = [0; 2];

    let _ = reader.read(&mut buffer);
}

#[test]
fn test_tee_reader_returns_branch_write_error() {
    let source = ScriptedReader::bytes(b"abc");
    let mut reader =
        TeeReader::new(source, ScriptedBranch::failing("branch write failed"));
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("branch write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch write failed", error.to_string());
}

#[test]
fn test_tee_reader_forwards_seek_to_source_reader() {
    use std::io::Cursor;

    let source = Cursor::new(b"abcdef".to_vec());
    let branch = Vec::new();
    let mut reader = TeeReader::new(source, branch);

    reader
        .seek(SeekFrom::Start(2))
        .expect("seek should be forwarded");
    let mut buffer = [0; 2];
    reader
        .read_exact(&mut buffer)
        .expect("read after seek should succeed");

    assert_eq!(b"cd", &buffer);
    assert_eq!(b"cd", reader.branch_ref().as_slice());
}

#[test]
fn test_sync_seek_tee_reader_seeks_source_and_branch() {
    use std::io::Cursor;

    let source = Cursor::new(b"abcdef".to_vec());
    let branch = Cursor::new(vec![0; 6]);
    let mut reader = TeeReader::with_sync_branch_seek(source, branch);

    let position = reader
        .seek(SeekFrom::Start(2))
        .expect("sync tee seek should move both sides");
    let mut buffer = [0; 2];
    reader
        .read_exact(&mut buffer)
        .expect("read after sync seek should succeed");

    assert_eq!(2, position);
    assert_eq!(b"cd", &buffer);
    assert_eq!(4, reader.reader_ref().position());
    assert_eq!(4, reader.branch_ref().position());
    assert_eq!(
        &[0, 0, b'c', b'd', 0, 0],
        reader.branch_ref().get_ref().as_slice()
    );
}

#[test]
fn test_sync_seek_tee_reader_source_seek_error_does_not_seek_branch() {
    let source = ScriptedSeek::failing(3, "source seek failed");
    let branch = ScriptedSeek::new(5);
    let mut reader = TeeReader::with_sync_branch_seek(source, branch);

    let error = reader
        .seek(SeekFrom::Start(7))
        .expect_err("source seek error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("source seek failed", error.to_string());
    assert_eq!(1, reader.reader_ref().seek_calls);
    assert_eq!(3, reader.reader_ref().position);
    assert_eq!(0, reader.branch_ref().seek_calls);
    assert_eq!(5, reader.branch_ref().position);
}

#[test]
fn test_sync_seek_tee_reader_branch_seek_error_leaves_source_moved() {
    let source = ScriptedSeek::new(3);
    let branch = ScriptedSeek::failing(5, "branch seek failed");
    let mut reader = TeeReader::with_sync_branch_seek(source, branch);

    let error = reader
        .seek(SeekFrom::Start(7))
        .expect_err("branch seek error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch seek failed", error.to_string());
    assert_eq!(1, reader.reader_ref().seek_calls);
    assert_eq!(7, reader.reader_ref().position);
    assert_eq!(1, reader.branch_ref().seek_calls);
    assert_eq!(5, reader.branch_ref().position);
}

#[test]
fn test_sync_seek_tee_reader_new_exposes_wrapped_streams() {
    use std::io::Cursor;

    let source = Cursor::new(b"abc".to_vec());
    let branch = Cursor::new(Vec::<u8>::new());
    let reader = SyncSeekTeeReader::new(source, branch);

    assert_eq!(b"abc", reader.reader_ref().get_ref().as_slice());
    assert!(reader.branch_ref().get_ref().is_empty());
}
