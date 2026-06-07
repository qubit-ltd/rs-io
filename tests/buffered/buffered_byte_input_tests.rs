// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
    Seek,
    SeekFrom,
};

use qubit_io::buffered::BufferedByteInput;

enum ReadStep {
    Data(Vec<u8>),
    Interrupted,
    Error(ErrorKind, &'static str),
    Eof,
}

struct ScriptedReader {
    steps: VecDeque<ReadStep>,
}

impl ScriptedReader {
    fn new(steps: Vec<ReadStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
        }
    }
}

impl Read for ScriptedReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(data) => {
                let count = data.len().min(output.len());
                output[..count].copy_from_slice(&data[..count]);
                if count < data.len() {
                    self.steps
                        .push_front(ReadStep::Data(data[count..].to_vec()));
                }
                Ok(count)
            }
            ReadStep::Interrupted => {
                Err(Error::new(ErrorKind::Interrupted, "interrupted"))
            }
            ReadStep::Error(kind, message) => Err(Error::new(kind, message)),
            ReadStep::Eof => Ok(0),
        }
    }
}

struct PanicOnRead;

impl Read for PanicOnRead {
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        panic!("read should not be called")
    }
}

struct FailingSeekReader;

impl Read for FailingSeekReader {
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Seek for FailingSeekReader {
    fn seek(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Err(Error::other("seek failed"))
    }
}

#[test]
fn test_new_and_accessors_expose_inner_reader() {
    let mut input = BufferedByteInput::new(Cursor::new(b"abc".to_vec()));

    assert_eq!(0, input.available());
    assert_eq!(0, input.inner().position());

    input.inner_mut().set_position(2);
    let cursor = input.into_inner();

    assert_eq!(2, cursor.position());
}

#[test]
fn test_capacity_returns_internal_buffer_capacity() {
    let cursor = Cursor::new(b"abc".to_vec());
    let input = BufferedByteInput::with_capacity(cursor, 4);

    assert_eq!(4, input.capacity());
}

#[test]
#[should_panic(expected = "cannot consume beyond buffered input")]
fn test_consume_panics_when_count_exceeds_available() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);

    input.consume(1);
}

#[test]
fn test_consume_unchecked_advances_without_bounds_check() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    // SAFETY: The buffer has four readable bytes, so consuming two is valid.
    unsafe {
        input.consume_unchecked(2);
    }

    assert_eq!(b"cd", input.unread_slice());
}

#[test]
fn test_fill_more_preserves_unread_tail_and_appends_new_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    assert_eq!(b"abcd", input.unread_slice());

    input.consume(2);
    assert_eq!(b"cd", input.unread_slice());

    assert!(input.fill_more().expect("second refill should succeed"));
    assert_eq!(b"cdef", input.unread_slice());
}

#[test]
fn test_fill_until_buffers_requested_available_bytes() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"abc".to_vec()),
        ReadStep::Data(b"de".to_vec()),
    ]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    assert!(
        input.fill_until(4).expect(
            "fill_until should read until requested bytes are buffered"
        )
    );

    assert_eq!(b"bcde", input.unread_slice());
}

#[test]
fn test_fill_until_returns_false_when_eof_prevents_requested_bytes() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"ab".to_vec()),
        ReadStep::Eof,
    ]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);

    assert!(
        !input
            .fill_until(3)
            .expect("EOF before requested count should not be an I/O error")
    );

    assert_eq!(b"ab", input.unread_slice());
}

#[test]
fn test_fill_until_rejects_count_exceeding_capacity() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);

    let error = input
        .fill_until(5)
        .expect_err("count beyond capacity should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_ensure_available_returns_unexpected_eof_and_consumes_partial_bytes() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"ab".to_vec()),
        ReadStep::Eof,
    ]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);

    let error = input
        .ensure_available(3)
        .expect_err("ensure_available should require the full byte count");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(0, input.available());
}

#[test]
fn test_fill_more_returns_false_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);

    assert!(!input.fill_more().expect("EOF refill should succeed"));
    assert_eq!(0, input.available());
}

#[test]
fn test_fill_more_retries_interrupted_reads() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Interrupted,
        ReadStep::Data(b"ab".to_vec()),
    ]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);

    assert!(
        input
            .fill_more()
            .expect("interrupted read should be retried")
    );
    assert_eq!(b"ab", input.unread_slice());
}

#[test]
fn test_fill_more_appends_when_tail_capacity_remains() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"ab".to_vec()),
        ReadStep::Data(b"cd".to_vec()),
    ]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    assert!(input.fill_more().expect("second refill should append"));

    assert_eq!(b"bcd", input.unread_slice());
}

#[test]
fn test_fill_more_returns_non_interrupted_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "read failed",
    )]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);

    let error = input
        .fill_more()
        .expect_err("non-interrupted read error should be returned");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_forwards_through_buffered_input() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    let mut output = [0_u8; 3];
    let count = input
        .read(output.as_mut_slice())
        .expect("buffered read should succeed");

    assert_eq!(3, count);
    assert_eq!(b"bcd", &output);
}

#[test]
fn test_read_into_unchecked_writes_at_output_index_and_count() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);
    let mut output = [b'.'; 6];

    // SAFETY: `output[2..5]` is a valid writable range.
    let count = unsafe {
        input
            .read_into_unchecked(output.as_mut_slice(), 2, 3)
            .expect("indexed unchecked read should succeed")
    };

    assert_eq!(3, count);
    assert_eq!(b"..bcd.", &output);
    assert_eq!(0, input.available());
}

#[test]
fn test_read_empty_output_does_not_read() {
    let mut input = BufferedByteInput::with_capacity(PanicOnRead, 4);
    let mut output = [];

    let count = input
        .read(output.as_mut_slice())
        .expect("empty output should be accepted");

    assert_eq!(0, count);
}

#[test]
fn test_read_delegates_large_empty_buffer_read() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 6];

    let count = input
        .read(output.as_mut_slice())
        .expect("large read should be delegated");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", &output);
}

#[test]
fn test_read_refills_small_empty_buffer_read() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 2];

    let count = input
        .read(output.as_mut_slice())
        .expect("small read should refill the internal buffer");

    assert_eq!(2, count);
    assert_eq!(b"ab", &output);
    assert_eq!(b"cd", input.unread_slice());
}

#[test]
fn test_read_returns_zero_when_small_read_reaches_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 2];

    let count = input
        .read(output.as_mut_slice())
        .expect("EOF should be reported as zero bytes read");

    assert_eq!(0, count);
}

#[test]
fn test_read_returns_refill_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "refill failed",
    )]);
    let mut input = BufferedByteInput::with_capacity(reader, 4);
    let mut output = [0_u8; 2];

    let error = input
        .read(output.as_mut_slice())
        .expect_err("refill error should be returned");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("refill failed", error.to_string());
}

#[test]
fn test_seek_current_accounts_for_prefetched_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    let position = input
        .stream_position()
        .expect("seek should use logical position");

    assert_eq!(1, position);
}

#[test]
fn test_seek_rejects_current_offset_underflow() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let error = input
        .seek(SeekFrom::Current(i64::MIN))
        .expect_err("underflowing adjusted offset should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_seek_accepts_absolute_position_and_discards_buffer() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedByteInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let position = input
        .seek(SeekFrom::Start(3))
        .expect("absolute seek should succeed");
    let mut output = [0_u8; 2];
    let count = input
        .read(output.as_mut_slice())
        .expect("read after seek should succeed");

    assert_eq!(3, position);
    assert_eq!(2, count);
    assert_eq!(b"de", &output);
}

#[test]
fn test_seek_returns_underlying_seek_error() {
    let mut input = BufferedByteInput::with_capacity(FailingSeekReader, 4);

    let error = input
        .seek(SeekFrom::Start(0))
        .expect_err("underlying seek error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("seek failed", error.to_string());
}
