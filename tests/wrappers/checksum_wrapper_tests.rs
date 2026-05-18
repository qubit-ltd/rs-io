/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
    Write,
};

use qubit_io::{
    ChecksumReader,
    ChecksumWriter,
};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct ShortWriter {
    data: Vec<u8>,
    max_chunk: usize,
}

impl ShortWriter {
    fn new(max_chunk: usize) -> Self {
        Self {
            data: Vec::new(),
            max_chunk,
        }
    }
}

impl Write for ShortWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let count = buffer.len().min(self.max_chunk);
        self.data.extend_from_slice(&buffer[..count]);
        Ok(count)
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
        Err(Error::other("flush failed"))
    }
}

fn expected_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[test]
fn test_checksum_reader_hashes_successfully_read_bytes() {
    let source = Cursor::new(b"abcdef".to_vec());
    let mut reader = ChecksumReader::new(source, DefaultHasher::new());
    assert_eq!(expected_checksum(b""), reader.checksum());

    let mut buffer = [0; 3];
    let count = reader
        .read(&mut buffer)
        .expect("checksum read should succeed");

    assert_eq!(3, count);
    assert_eq!(expected_checksum(b"abc"), reader.checksum());
    assert_eq!(3, reader.get_ref().position());

    let (source, hasher) = reader.into_inner();
    assert_eq!(3, source.position());
    assert_eq!(expected_checksum(b"abc"), hasher.finish());
}

#[test]
fn test_checksum_reader_does_not_hash_failed_reads() {
    let mut reader = ChecksumReader::new(FailingReader, DefaultHasher::new());
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(expected_checksum(b""), reader.checksum());
}

#[test]
fn test_checksum_reader_mut_accessors_allow_inner_and_hasher_access() {
    let source = Cursor::new(b"abc".to_vec());
    let mut reader = ChecksumReader::new(source, DefaultHasher::new());

    reader.get_mut().set_position(1);
    reader.hasher_mut().write(b"x");
    let mut buffer = [0; 2];
    reader.read_exact(&mut buffer).unwrap();

    assert_eq!(expected_checksum(b"xbc"), reader.checksum());
    assert_eq!(expected_checksum(b"xbc"), reader.hasher_ref().finish());
}

#[test]
fn test_checksum_writer_hashes_successfully_written_bytes() {
    let primary = ShortWriter::new(3);
    let mut writer = ChecksumWriter::new(primary, DefaultHasher::new());
    assert_eq!(expected_checksum(b""), writer.checksum());

    let count = writer
        .write(b"abcdef")
        .expect("checksum write should succeed");

    assert_eq!(3, count);
    assert_eq!(b"abc", writer.get_ref().data.as_slice());
    assert_eq!(expected_checksum(b"abc"), writer.checksum());

    let (primary, hasher) = writer.into_inner();
    assert_eq!(b"abc", primary.data.as_slice());
    assert_eq!(expected_checksum(b"abc"), hasher.finish());
}

#[test]
fn test_checksum_writer_does_not_hash_failed_writes() {
    let mut writer = ChecksumWriter::new(FailingWriter, DefaultHasher::new());

    let error = writer
        .write(b"abc")
        .expect_err("write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert_eq!(expected_checksum(b""), writer.checksum());
}

#[test]
fn test_checksum_writer_mut_accessors_and_flush_delegate_to_inner_writer() {
    let primary = ShortWriter::new(4);
    let mut writer = ChecksumWriter::new(primary, DefaultHasher::new());

    writer.get_mut().data.extend_from_slice(b"x");
    writer.hasher_mut().write(b"y");
    writer.write_all(b"ab").unwrap();
    writer.flush().unwrap();

    assert_eq!(b"xab", writer.get_ref().data.as_slice());
    assert_eq!(expected_checksum(b"yab"), writer.hasher_ref().finish());
}

#[test]
fn test_checksum_writer_returns_flush_error() {
    let mut writer = ChecksumWriter::new(FailingWriter, DefaultHasher::new());

    let error = writer.flush().expect_err("flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
}
