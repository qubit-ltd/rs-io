// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Read, Write};

use qubit_io::TeeReader;

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
    let mut reader = TeeReader::new(ScriptedReader::error("read failed"), ScriptedBranch::new());
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
    let mut reader = TeeReader::new(ScriptedReader::invalid_count(), ScriptedBranch::new());
    let mut buffer = [0; 2];

    let _ = reader.read(&mut buffer);
}

#[test]
fn test_tee_reader_returns_branch_write_error() {
    let source = ScriptedReader::bytes(b"abc");
    let mut reader = TeeReader::new(source, ScriptedBranch::failing("branch write failed"));
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("branch write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch write failed", error.to_string());
}

#[test]
fn test_tee_reader_forwards_seek_to_source_reader() {
    use std::io::{Cursor, Seek, SeekFrom};

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
