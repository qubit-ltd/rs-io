// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::{
    BufRead,
    Cursor,
    Error,
    ErrorKind,
    Read,
    Seek,
    SeekFrom,
};

use qubit_io::{
    BufferedInput,
    Input,
};

struct U16Input {
    chunks: VecDeque<Vec<u16>>,
}

impl U16Input {
    fn new(chunks: Vec<Vec<u16>>) -> Self {
        Self {
            chunks: VecDeque::from(chunks),
        }
    }
}

impl Input for U16Input {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        let Some(chunk) = self.chunks.pop_front() else {
            return Ok(0);
        };
        let read = count.min(chunk.len());
        output[index..index + read].copy_from_slice(&chunk[..read]);
        if read < chunk.len() {
            self.chunks.push_front(chunk[read..].to_vec());
        }
        Ok(read)
    }
}

struct OverreportingInput;

impl Input for OverreportingInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u16],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
    }
}

#[test]
fn test_buffered_input_reads_generic_units() {
    let inner = U16Input::new(vec![vec![1, 2, 3], vec![4, 5]]);
    let mut input = BufferedInput::with_capacity(inner, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    assert_eq!(&[1, 2, 3], input.unread_slice());
    input.consume(1);

    assert!(input.fill_until(4).expect("refill should append units"));
    assert_eq!(&[2, 3, 4, 5], input.unread_slice());

    let mut output = [0_u16; 3];
    // SAFETY: `output[0..3]` is a valid destination range.
    let read = unsafe {
        input
            .read_into_unchecked(&mut output, 0, 3)
            .expect("buffered read should succeed")
    };

    assert_eq!(3, read);
    assert_eq!([2, 3, 4], output);
    assert_eq!(&[5], input.unread_slice());
}

#[test]
fn test_buffered_input_adapts_std_read_as_u8_input() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 5];

    // SAFETY: `output[1..4]` is a valid destination range.
    let read = unsafe {
        input
            .read_into_unchecked(&mut output, 1, 3)
            .expect("std reader should be an Input<Item = u8>")
    };

    assert_eq!(3, read);
    assert_eq!([0, b'a', b'b', b'c', 0], output);
}

#[test]
fn test_buffered_input_rejects_overreported_unit_count() {
    let mut input = BufferedInput::with_capacity(OverreportingInput, 4);

    let error = input
        .fill_more()
        .expect_err("overreported read count should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_input_u8_blanket_impl_reuses_std_read_errors() {
    struct FailingReader;

    impl std::io::Read for FailingReader {
        fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
            Err(Error::other("read failed"))
        }
    }

    let mut reader = FailingReader;
    let mut output = [0_u8; 1];

    // SAFETY: The full output range is valid.
    let error =
        unsafe { Input::read_unchecked(&mut reader, &mut output, 0, 1) }
            .expect_err("std read error should be propagated");

    assert_eq!(ErrorKind::Other, error.kind());
}
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

struct OverreportingReader;

impl Read for OverreportingReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        Ok(output.len() + 1)
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
    let mut input = BufferedInput::new(Cursor::new(b"abc".to_vec()));

    assert_eq!(0, input.available());
    assert_eq!(0, input.inner().position());

    input.inner_mut().set_position(2);
    let (cursor, unread) = input.into_parts();

    assert_eq!(2, cursor.position());
    assert!(unread.is_empty());
}

#[test]
fn test_capacity_returns_internal_buffer_capacity() {
    let cursor = Cursor::new(b"abc".to_vec());
    let input = BufferedInput::with_capacity(cursor, 4);

    assert_eq!(4, input.capacity());
}

#[test]
#[should_panic(expected = "cannot consume beyond buffered input")]
fn test_consume_panics_when_count_exceeds_available() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    input.consume(1);
}

#[test]
fn test_consume_unchecked_advances_without_bounds_check() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    // SAFETY: The buffer has four readable bytes, so consuming two is valid.
    unsafe {
        input.consume_unchecked(2);
    }

    assert_eq!(b"cd", input.unread_slice());
}

#[test]
fn test_into_parts_returns_inner_and_unread_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    let (cursor, unread) = input.into_parts();

    assert_eq!(4, cursor.position());
    assert_eq!(b"bcd", unread.as_slice());
}

#[test]
fn test_unread_raw_parts_exposes_backing_buffer_index_and_count() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    input.consume(1);

    let (buffer, index, count) = input.unread_raw_parts();

    assert_eq!(1, index);
    assert_eq!(3, count);
    assert_eq!(b"bcd", &buffer[index..index + count]);
}

#[test]
fn test_buf_read_fill_buf_exposes_unread_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert_eq!(
        b"abcd",
        input
            .fill_buf()
            .expect("fill_buf should refill from wrapped reader")
    );
    BufRead::consume(&mut input, 2);
    assert_eq!(
        b"cd",
        input
            .fill_buf()
            .expect("fill_buf should reuse buffered bytes")
    );
    BufRead::consume(&mut input, 2);
    assert_eq!(
        b"ef",
        input
            .fill_buf()
            .expect("fill_buf should refill after consumption")
    );
}

#[test]
fn test_fill_more_preserves_unread_tail_and_appends_new_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

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
    let mut input = BufferedInput::with_capacity(reader, 4);
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
    let mut input = BufferedInput::with_capacity(reader, 4);

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
    let mut input = BufferedInput::with_capacity(cursor, 4);

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
    let mut input = BufferedInput::with_capacity(reader, 4);

    let error = input
        .ensure_available(3)
        .expect_err("ensure_available should require the full byte count");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(0, input.available());
}

#[test]
fn test_ensure_available_succeeds_when_requested_bytes_are_buffered() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    input
        .ensure_available(3)
        .expect("ensure_available should succeed with enough bytes");

    assert_eq!(b"abcd", input.unread_slice());
}

#[test]
fn test_fill_more_returns_false_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(!input.fill_more().expect("EOF refill should succeed"));
    assert_eq!(0, input.available());
}

#[test]
fn test_buf_read_fill_buf_returns_empty_slice_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert_eq!(
        b"",
        input.fill_buf().expect("fill_buf at EOF should succeed")
    );
}

#[test]
fn test_fill_more_retries_interrupted_reads() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Interrupted,
        ReadStep::Data(b"ab".to_vec()),
    ]);
    let mut input = BufferedInput::with_capacity(reader, 4);

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
    let mut input = BufferedInput::with_capacity(reader, 4);
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
    let mut input = BufferedInput::with_capacity(reader, 4);

    let error = input
        .fill_more()
        .expect_err("non-interrupted read error should be returned");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_forwards_through_buffered_input() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(PanicOnRead, 4);
    let mut output = [];

    let count = input
        .read(output.as_mut_slice())
        .expect("empty output should be accepted");

    assert_eq!(0, count);
}

#[test]
fn test_read_delegates_large_empty_buffer_read() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 6];

    let count = input
        .read(output.as_mut_slice())
        .expect("large read should be delegated");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", &output);
}

#[test]
fn test_read_delegated_large_empty_buffer_returns_zero_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    let mut output = [1_u8; 4];

    let count = input
        .read(output.as_mut_slice())
        .expect("delegated EOF read should succeed");

    assert_eq!(0, count);
    assert_eq!([1, 1, 1, 1], output);
}

#[test]
fn test_read_refills_small_empty_buffer_read() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(reader, 4);
    let mut output = [0_u8; 2];

    let error = input
        .read(output.as_mut_slice())
        .expect_err("refill error should be returned");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("refill failed", error.to_string());
}

#[test]
fn test_fill_more_rejects_invalid_read_count() {
    let mut input = BufferedInput::with_capacity(OverreportingReader, 4);

    let error = input
        .fill_more()
        .expect_err("overreported read count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_rejects_invalid_delegated_read_count() {
    let mut input = BufferedInput::with_capacity(OverreportingReader, 4);
    let mut output = [0_u8; 4];

    let error = input
        .read(output.as_mut_slice())
        .expect_err("overreported delegated read count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_seek_current_accounts_for_prefetched_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let error = input
        .seek(SeekFrom::Current(i64::MIN))
        .expect_err("underflowing adjusted offset should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_seek_accepts_absolute_position_and_discards_buffer() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
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
    let mut input = BufferedInput::with_capacity(FailingSeekReader, 4);

    let error = input
        .seek(SeekFrom::Start(0))
        .expect_err("underlying seek error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("seek failed", error.to_string());
}
