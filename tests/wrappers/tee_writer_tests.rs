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
    Error,
    ErrorKind,
    Write,
};

use qubit_io::TeeWriter;

struct ScriptedWriter {
    data: Vec<u8>,
    max_chunk: usize,
    write_error: Option<&'static str>,
    flush_error: Option<&'static str>,
    invalid_count: bool,
}

impl ScriptedWriter {
    fn accepting() -> Self {
        Self::short(usize::MAX)
    }

    fn short(max_chunk: usize) -> Self {
        Self {
            data: Vec::new(),
            max_chunk,
            write_error: None,
            flush_error: None,
            invalid_count: false,
        }
    }

    fn failing_write(message: &'static str) -> Self {
        Self {
            write_error: Some(message),
            ..Self::accepting()
        }
    }

    fn failing_flush(message: &'static str) -> Self {
        Self {
            flush_error: Some(message),
            ..Self::accepting()
        }
    }

    fn invalid_count() -> Self {
        Self {
            invalid_count: true,
            ..Self::accepting()
        }
    }

    fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }
}

impl Write for ScriptedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if let Some(message) = self.write_error {
            return Err(Error::other(message));
        }
        if self.invalid_count {
            return Ok(buffer.len() + 1);
        }
        let count = buffer.len().min(self.max_chunk);
        self.data.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(message) = self.flush_error {
            return Err(Error::other(message));
        }
        Ok(())
    }
}

#[test]
fn test_tee_writer_copies_written_bytes_to_branch_writer() {
    let primary = ScriptedWriter::short(3);
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);
    assert_eq!(0, writer.primary_ref().data.len());
    assert!(writer.branch_ref().as_slice().is_empty());

    let count = writer.write(b"abcdef").expect("tee write should succeed");

    assert_eq!(3, count);
    assert_eq!(b"abc", writer.primary_ref().as_slice());
    assert_eq!(b"abc", writer.branch_ref().as_slice());

    let (primary, branch) = writer.into_inner();
    assert_eq!(b"abc", primary.as_slice());
    assert_eq!(b"abc", branch.as_slice());
}

#[test]
fn test_tee_writer_mut_accessors_allow_inner_access() {
    let primary = ScriptedWriter::short(4);
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    writer.primary_mut().data.extend_from_slice(b"x");
    writer.branch_mut().data.extend_from_slice(b"y");
    let count = writer.write(b"ab").expect("tee write should succeed");

    assert_eq!(2, count);
    assert_eq!(b"xab", writer.primary_ref().as_slice());
    assert_eq!(b"yab", writer.branch_ref().as_slice());
}

#[test]
fn test_tee_writer_allows_primary_zero_length_write() {
    let primary = ScriptedWriter::short(0);
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    let count = writer
        .write(b"ab")
        .expect("zero-length primary write should succeed");

    assert_eq!(0, count);
    assert!(writer.primary_ref().as_slice().is_empty());
    assert!(writer.branch_ref().as_slice().is_empty());
}

#[test]
fn test_tee_writer_returns_primary_write_error() {
    let primary = ScriptedWriter::failing_write("primary write failed");
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .write(b"ab")
        .expect_err("primary write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("primary write failed", error.to_string());
    assert!(writer.branch_ref().as_slice().is_empty());
}

#[test]
#[should_panic]
fn test_tee_writer_panics_when_primary_returns_invalid_count() {
    let primary = ScriptedWriter::invalid_count();
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    let _ = writer.write(b"ab");
}

#[test]
fn test_tee_writer_returns_branch_write_error() {
    let primary = ScriptedWriter::short(4);
    let branch = ScriptedWriter::failing_write("branch write failed");
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .write(b"ab")
        .expect_err("branch write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch write failed", error.to_string());
    assert_eq!(b"ab", writer.primary_ref().as_slice());
}

#[test]
fn test_tee_writer_flushes_both_writers() {
    let primary = ScriptedWriter::accepting();
    let branch = ScriptedWriter::failing_flush("branch flush failed");
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .flush()
        .expect_err("branch flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("branch flush failed", error.to_string());
}

#[test]
fn test_tee_writer_flush_succeeds_when_both_writers_flush() {
    let primary = ScriptedWriter::accepting();
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    writer.flush().expect("flush should succeed");
}

#[test]
fn test_tee_writer_returns_primary_flush_error() {
    let primary = ScriptedWriter::failing_flush("primary flush failed");
    let branch = ScriptedWriter::accepting();
    let mut writer = TeeWriter::new(primary, branch);

    let error = writer
        .flush()
        .expect_err("primary flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("primary flush failed", error.to_string());
    assert!(writer.branch_ref().as_slice().is_empty());
}

#[test]
fn test_tee_writer_forwards_seek_to_both_writers() {
    use std::io::{
        Cursor,
        Seek,
        SeekFrom,
    };

    let primary = Cursor::new(Vec::new());
    let branch = Cursor::new(Vec::new());
    let mut writer = TeeWriter::new(primary, branch);

    writer.write_all(b"abc").unwrap();
    writer
        .seek(SeekFrom::Start(1))
        .expect("seek should be forwarded");
    writer.write_all(b"z").unwrap();

    let (primary, branch) = writer.into_inner();
    assert_eq!(b"azc", primary.into_inner().as_slice());
    assert_eq!(b"azc", branch.into_inner().as_slice());
}
