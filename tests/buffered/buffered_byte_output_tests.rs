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
    Seek,
    SeekFrom,
    Write,
};

use qubit_io::buffered::BufferedByteOutput;

#[derive(Debug)]
enum WriteStep {
    Accept(usize),
    Interrupted,
    Error(ErrorKind, &'static str),
    Zero,
}

#[derive(Debug)]
struct ScriptedWriter {
    output: Vec<u8>,
    steps: VecDeque<WriteStep>,
    fail_flush: bool,
}

impl ScriptedWriter {
    fn new(steps: Vec<WriteStep>) -> Self {
        Self {
            output: Vec::new(),
            steps: VecDeque::from(steps),
            fail_flush: false,
        }
    }

    fn with_flush_error() -> Self {
        Self {
            output: Vec::new(),
            steps: VecDeque::new(),
            fail_flush: true,
        }
    }
}

struct OverreportingWriter;

impl Write for OverreportingWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        Ok(input.len() + 1)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for ScriptedWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        match self
            .steps
            .pop_front()
            .unwrap_or(WriteStep::Accept(input.len()))
        {
            WriteStep::Accept(max_count) => {
                let count = input.len().min(max_count);
                self.output.extend_from_slice(&input[..count]);
                Ok(count)
            }
            WriteStep::Interrupted => {
                Err(Error::new(ErrorKind::Interrupted, "interrupted"))
            }
            WriteStep::Error(kind, message) => Err(Error::new(kind, message)),
            WriteStep::Zero => Ok(0),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush {
            Err(Error::other("flush failed"))
        } else {
            Ok(())
        }
    }
}

fn flush_into_inner<W>(mut output: BufferedByteOutput<W>) -> W
where
    W: Write,
{
    output.flush().expect("flush should succeed");
    let (inner, pending) = output.into_parts();
    assert!(
        pending.is_empty(),
        "successful flush should leave no pending bytes"
    );
    inner
}

#[test]
fn test_new_and_inner_mut_expose_writer() {
    let mut output = BufferedByteOutput::new(Cursor::new(Vec::new()));

    output
        .inner_mut()
        .write_all(b"raw")
        .expect("inner writer should be mutable");
    let cursor = flush_into_inner(output);

    assert_eq!(b"raw", cursor.into_inner().as_slice());
}

#[test]
fn test_capacity_returns_internal_buffer_capacity() {
    let output =
        BufferedByteOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);

    assert_eq!(4, output.capacity());
}

#[test]
#[should_panic(expected = "cannot advance beyond spare output buffer")]
fn test_advance_panics_when_count_exceeds_spare_capacity() {
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    output.advance(5);
}

#[test]
fn test_spare_buffer_mut_and_advance_append_to_buffer() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    output
        .write_all(b"ab")
        .expect("buffered write should succeed");
    {
        let spare = output.spare_buffer_mut();
        spare[..2].copy_from_slice(b"cd");
    }
    output.advance(2);

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_advance_unchecked_marks_spare_bytes_as_written() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    output.spare_buffer_mut()[0..2].copy_from_slice(b"ab");
    // SAFETY: Two bytes were initialized in the spare buffer, and the spare
    // capacity is four bytes.
    unsafe {
        output.advance_unchecked(2);
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"ab", cursor.into_inner().as_slice());
}

#[test]
fn test_spare_raw_parts_mut_exposes_backing_buffer_index_and_count() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"ab")
        .expect("buffered write should succeed");

    let (buffer, index, count) = output.spare_raw_parts_mut();
    assert_eq!(2, index);
    assert_eq!(2, count);
    buffer[index..index + count].copy_from_slice(b"cd");
    // SAFETY: Two bytes were initialized in the spare range reported by
    // `spare_raw_parts_mut`.
    unsafe {
        output.advance_unchecked(count);
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_ensure_spare_capacity_rejects_oversized_request() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    let error = output
        .ensure_spare_capacity(5)
        .expect_err("oversized spare request should fail");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_ensure_spare_capacity_succeeds_without_flushing_when_space_remains() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"a")
        .expect("buffered write should succeed");

    output
        .ensure_spare_capacity(2)
        .expect("existing spare capacity should be enough");

    assert_eq!(3, output.spare_capacity());
    assert!(output.inner().get_ref().is_empty());
}

#[test]
fn test_ensure_spare_capacity_flushes_when_spare_is_too_small() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    output
        .ensure_spare_capacity(2)
        .expect("buffer should be flushed to make room");

    assert_eq!(4, output.spare_capacity());
    assert_eq!(b"abc", output.inner().get_ref().as_slice());
}

#[test]
fn test_ensure_spare_capacity_returns_flush_error() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .ensure_spare_capacity(2)
        .expect_err("flush error should be returned");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_write_all_delegates_large_empty_buffer_write() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    output
        .write_all(b"abcd")
        .expect("large write should be delegated");
    let cursor = flush_into_inner(output);

    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_write_all_delegated_large_write_retries_interrupted_writer() {
    let writer =
        ScriptedWriter::new(vec![WriteStep::Interrupted, WriteStep::Accept(4)]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);

    output
        .write_all(b"abcd")
        .expect("interrupted delegated write_all should be retried");

    assert_eq!(b"abcd", output.inner().output.as_slice());
}

#[test]
fn test_write_all_delegated_large_write_returns_write_zero() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);

    let error = output
        .write_all(b"abcd")
        .expect_err("zero-length delegated write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_write_all_delegated_large_write_returns_writer_error() {
    let writer = ScriptedWriter::new(vec![WriteStep::Error(
        ErrorKind::BrokenPipe,
        "write failed",
    )]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);

    let error = output
        .write_all(b"abcd")
        .expect_err("delegated writer error should be returned");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_write_all_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"abc")
        .expect("buffered prefix should be accepted");

    output
        .write_all(b"xy")
        .expect("small write should flush prefix and then buffer");
    assert_eq!(b"abc", output.inner().get_ref().as_slice());

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcxy", cursor.into_inner().as_slice());
}

#[test]
fn test_write_delegates_large_empty_buffer_write() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    let count = output
        .write(b"abcd")
        .expect("large raw write should be delegated");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.inner().get_ref().as_slice());
}

#[test]
fn test_write_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    let prefix_count = output
        .write(b"abc")
        .expect("buffered prefix should be accepted");
    assert_eq!(3, prefix_count);

    let count = output
        .write(b"xy")
        .expect("small raw write should flush prefix and then buffer");
    assert_eq!(2, count);
    assert_eq!(b"abc", output.inner().get_ref().as_slice());

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcxy", cursor.into_inner().as_slice());
}

#[test]
fn test_flush_buffer_accepts_empty_buffer() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    output
        .flush_buffer()
        .expect("empty flush should complete without writing");

    assert!(output.inner().get_ref().is_empty());
}

#[test]
fn test_flush_buffer_retries_interrupted_writes() {
    let writer =
        ScriptedWriter::new(vec![WriteStep::Interrupted, WriteStep::Accept(4)]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    output
        .flush_buffer()
        .expect("interrupted write should be retried");

    assert_eq!(b"abc", output.inner().output.as_slice());
}

#[test]
fn test_flush_buffer_returns_write_zero() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .flush_buffer()
        .expect_err("zero-length write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_flush_buffer_preserves_unwritten_suffix_after_error() {
    let writer = ScriptedWriter::new(vec![
        WriteStep::Accept(2),
        WriteStep::Error(ErrorKind::BrokenPipe, "write failed"),
    ]);
    let mut output = BufferedByteOutput::with_capacity(writer, 8);
    output
        .write_all(b"abcd")
        .expect("buffered write should succeed");

    let error = output
        .flush_buffer()
        .expect_err("writer error should be returned");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!(b"ab", output.inner().output.as_slice());

    output.inner_mut().steps.push_back(WriteStep::Accept(2));
    output
        .flush_buffer()
        .expect("remaining suffix should flush on retry");

    assert_eq!(b"abcd", output.inner().output.as_slice());
}

#[test]
fn test_flush_returns_inner_flush_error() {
    let writer = ScriptedWriter::with_flush_error();
    let mut output = BufferedByteOutput::with_capacity(writer, 4);

    let error = output
        .flush()
        .expect_err("inner flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
}

#[test]
fn test_write_trait_write_all_uses_buffered_write_all() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    Write::write_all(&mut output, b"abcd")
        .expect("write_all trait method should delegate");
    let cursor = flush_into_inner(output);

    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_seek_flushes_pending_bytes_before_seeking() {
    let cursor = Cursor::new(vec![0; 4]);
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"ab")
        .expect("buffered write should succeed");

    let position = output
        .seek(SeekFrom::Start(0))
        .expect("seek should flush pending bytes first");
    output
        .write_all(b"xy")
        .expect("second write should be buffered");
    let cursor = flush_into_inner(output);

    assert_eq!(0, position);
    assert_eq!(b"xy\0\0", cursor.into_inner().as_slice());
}

#[test]
fn test_write_forwards_through_buffered_output() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);

    let accepted = output.write(b"abc").expect("write should succeed");
    assert_eq!(3, accepted);

    output.flush().expect("flush should succeed");
    assert_eq!(b"abc", output.inner().get_ref().as_slice());
}

#[test]
fn test_into_parts_returns_inner_and_pending_bytes_without_flushing() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedByteOutput::with_capacity(cursor, 4);
    output
        .write_all(b"ab")
        .expect("buffered write should succeed");

    let (cursor, pending) = output.into_parts();

    assert!(cursor.into_inner().is_empty());
    assert_eq!(b"ab", pending.as_slice());
}

#[test]
fn test_flush_error_keeps_output_owned_by_caller() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedByteOutput::with_capacity(writer, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .flush()
        .expect_err("write-zero flush should leave output owned by caller");

    assert_eq!(ErrorKind::WriteZero, error.kind());
    output.inner_mut().steps.push_back(WriteStep::Accept(3));

    output
        .flush()
        .expect("retrying flush should write preserved bytes");
    let (writer, pending) = output.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"abc", writer.output.as_slice());
}

#[test]
fn test_inner_flush_error_keeps_output_owned_by_caller() {
    let writer = ScriptedWriter::with_flush_error();
    let mut output = BufferedByteOutput::with_capacity(writer, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .flush()
        .expect_err("inner flush error should leave output owned by caller");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
    let (writer, pending) = output.into_parts();

    assert!(pending.is_empty());
    assert_eq!(b"abc", writer.output.as_slice());
}

#[test]
fn test_flush_buffer_rejects_invalid_write_count() {
    let mut output = BufferedByteOutput::with_capacity(OverreportingWriter, 4);
    output
        .write_all(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .flush_buffer()
        .expect_err("overreported flush count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_write_rejects_invalid_delegated_write_count() {
    let mut output = BufferedByteOutput::with_capacity(OverreportingWriter, 4);

    let error = output
        .write(b"abcd")
        .expect_err("overreported delegated write count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}
