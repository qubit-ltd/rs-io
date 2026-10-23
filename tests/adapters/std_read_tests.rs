// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use qubit_io::ChecksumInput;
use qubit_io::Input;
use qubit_io::InputRef;
use qubit_io::LimitOutput;
use qubit_io::TeeInput;

#[test]
fn test_std_read_delegates_to_input() {
    let mut reader = Cursor::new(b"abcd").into_std_read();
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).expect("reader should fill output");
    assert_eq!(&bytes, b"abcd");
    assert_eq!(reader.into_inner().position(), 4);
}

#[test]
fn test_std_read_reports_tee_primary_progress_before_branch_failure() {
    let tee = TeeInput::new(Cursor::new(b"abcd"), LimitOutput::new(Vec::<u8>::new(), 2));
    let mut reader = qubit_io::StdRead::new(tee);
    let mut bytes = [0; 3];
    assert_eq!(std::io::Read::read(&mut reader, &mut bytes).unwrap(), 3);
    assert_eq!(&bytes, b"abc");
    let mut next = [9];
    assert_eq!(
        std::io::Read::read(&mut reader, &mut next).unwrap_err().kind(),
        std::io::ErrorKind::WriteZero
    );
    assert_eq!(next, [9]);
    assert_eq!(reader.get_ref().inner().position(), 3);
}

#[test]
fn test_into_std_read_preserves_checksum_state_and_stream_position() {
    let input = ChecksumInput::new(Cursor::new(b"abcd"), std::collections::hash_map::DefaultHasher::new());
    let mut reader = input.into_std_read();
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes).expect("reader should fill output");
    assert_eq!(&bytes, b"ab");
    let input = reader.into_inner();
    assert_eq!(2, input.inner().position());
    assert_ne!(0, input.checksum());
}

#[test]
fn test_into_std_read_supports_borrowed_and_dyn_inputs() {
    let mut cursor = Cursor::new(b"abc");
    {
        let borrowed = InputRef::new(&mut cursor).into_std_read();
        let mut reader = borrowed;
        let mut bytes = [0; 1];
        reader.read_exact(&mut bytes).expect("read should succeed");
        assert_eq!(&bytes, b"a");
        assert_eq!(2, reader.seek(SeekFrom::Current(1)).expect("seek should succeed"));
    }
    assert_eq!(2, cursor.position());

    let mut cursor = Cursor::new(b"dyn");
    let input: &mut dyn Input<Item = u8> = &mut cursor;
    let mut reader = InputRef::new(input).into_std_read();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).expect("read should succeed");
    assert_eq!(bytes, b"dyn");
}

#[test]
fn test_std_read_seek_forwards_position_and_errors() {
    let mut reader = Cursor::new(b"abcd").into_std_read();
    assert_eq!(3, reader.seek(SeekFrom::Start(3)).expect("seek should succeed"));

    let mut reader = ErrorSeekInput.into_std_read();
    assert_eq!("seek failed", reader.seek(SeekFrom::Start(1)).unwrap_err().to_string());
}

#[test]
fn test_std_read_is_not_seekable_without_inner_seekability() {
    fn assert_read<T: Read>(_: &T) {}
    let reader = qubit_io::StdRead::new(ReadOnlyInput);
    assert_read(&reader);
    let mut reader = qubit_io::StdRead::new(ReadOnlyInput);
    assert_eq!(
        std::io::ErrorKind::UnexpectedEof,
        reader.read_exact(&mut [0]).unwrap_err().kind()
    );
}

#[test]
fn test_into_std_read_retries_interrupted_and_handles_short_reads() {
    let input = ShortInput {
        bytes: b"abc".to_vec(),
        interrupted: true,
    };
    let mut reader = input.into_std_read();
    let mut bytes = [0; 3];
    reader.read_exact(&mut bytes).expect("read_exact should retry and fill");
    assert_eq!(&bytes, b"abc");
}

struct ReadOnlyInput;

impl Input for ReadOnlyInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, _count: usize) -> std::io::Result<usize> {
        Ok(0)
    }
}

struct ErrorSeekInput;

impl Input for ErrorSeekInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, _count: usize) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl qubit_io::Seekable for ErrorSeekInput {
    type Unit = u8;

    fn seek_to(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::other("seek failed"))
    }
}

struct ShortInput {
    bytes: Vec<u8>,
    interrupted: bool,
}

impl Input for ShortInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> std::io::Result<usize> {
        if self.interrupted {
            self.interrupted = false;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        let read = count.min(1).min(self.bytes.len());
        output[index..index + read].copy_from_slice(&self.bytes[..read]);
        self.bytes.drain(..read);
        Ok(read)
    }
}
