// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::{
    Cell,
    RefCell,
};
use std::collections::VecDeque;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    SeekFrom,
    Write,
};
use std::rc::Rc;

use qubit_io::{
    BufferedOutput,
    EnsuredBufferedOutput,
    Output,
    Seekable,
};

#[derive(Default)]
struct U16SeekOutput {
    values: Vec<u16>,
    position: usize,
}

impl U16SeekOutput {
    fn new(values: Vec<u16>) -> Self {
        Self {
            values,
            position: 0,
        }
    }
}

impl Output for U16SeekOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        let end = index.checked_add(count).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput, "write range overflow")
        })?;
        if end > input.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "write range exceeds source buffer",
            ));
        }
        let limit = self.position.checked_add(count).ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput, "position overflow")
        })?;
        if limit > self.values.len() {
            self.values.resize(limit, 0);
        }
        self.values[self.position..limit].copy_from_slice(&input[index..end]);
        self.position = limit;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl qubit_io::Seekable for U16SeekOutput {
    type Unit = u16;

    fn seek_to(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let current = i64::try_from(self.position)
            .expect("position always fits into i64 on current platforms");
        let target = match position {
            SeekFrom::Start(offset) => i64::try_from(offset).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "seek start offset overflow",
                )
            })?,
            SeekFrom::Current(offset) => {
                current.checked_add(offset).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "seek position overflows i64",
                    )
                })?
            }
            SeekFrom::End(offset) => {
                let end = i64::try_from(self.values.len()).map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "seek position overflow during end-based seek",
                    )
                })?;
                end.checked_add(offset).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "seek position underflow or overflow",
                    )
                })?
            }
        };

        if target < 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "seek position cannot be negative for item stream",
            ));
        }

        let target = usize::try_from(target).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "seek position overflow")
        })?;
        if target > self.values.len() {
            self.values.resize(target, 0);
        }
        self.position = target;
        Ok(target as u64)
    }
}

#[derive(Default)]
struct U16Output {
    values: Vec<u16>,
    flushed: bool,
}

impl Output for U16Output {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        self.values.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushed = true;
        Ok(())
    }
}

struct OverreportingOutput;

impl Output for OverreportingOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        _input: &[u16],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_buffered_output_reports_buffered() {
    let output = BufferedOutput::with_capacity(U16Output::default(), 4);

    assert!(output.is_buffered());
}

#[test]
fn test_buffered_output_ensure_wraps_unbuffered_output() {
    let output = U16Output::default();
    let mut output = BufferedOutput::ensure(output);

    assert!(matches!(output, EnsuredBufferedOutput::Buffered(_)));
    assert!(output.is_buffered());

    output
        .write_fully(&[1, 2])
        .expect("ensured buffered output should write items");
    output
        .flush()
        .expect("ensured buffered output should flush items");

    let inner = match output {
        EnsuredBufferedOutput::Buffered(output) => output
            .into_inner()
            .expect("buffered output should flush on into_inner"),
        EnsuredBufferedOutput::AlreadyBuffered(_) => {
            panic!("unbuffered output should have been wrapped")
        }
    };

    assert_eq!(inner.values, vec![1, 2]);
}

#[test]
fn test_buffered_output_ensure_keeps_buffered_output() {
    let output = BufferedOutput::new(U16Output::default());
    let output = BufferedOutput::ensure(output);

    assert!(matches!(output, EnsuredBufferedOutput::AlreadyBuffered(_)));
    let mut output = match output {
        EnsuredBufferedOutput::AlreadyBuffered(output) => output,
        EnsuredBufferedOutput::Buffered(_) => {
            panic!("buffered output should have been kept")
        }
    };

    output
        .write_fully(&[1, 2, 3])
        .expect("already buffered output should still write fully");
    output
        .flush()
        .expect("already buffered output should still flush");

    let inner = output
        .into_inner()
        .expect("buffered output should flush on into_inner");

    assert_eq!(inner.values, vec![1, 2, 3]);
}

#[test]
fn test_buffered_output_ensure_delegates_single_writes() {
    let output = U16Output::default();
    let mut output = BufferedOutput::ensure(output);
    let written = output
        .write(&[1, 2])
        .expect("wrapped output should support single writes");

    assert_eq!(written, 2);

    let output = U16Output::default();
    let mut output = EnsuredBufferedOutput::AlreadyBuffered(output);
    assert!(output.is_buffered());
    let written = output
        .write(&[3, 4])
        .expect("already buffered output should support single writes");

    assert_eq!(written, 2);
    output
        .write_fully(&[5, 6])
        .expect("already buffered output should support full writes");
    output
        .flush()
        .expect("already buffered output should support flushes");
}

#[test]
fn test_buffered_output_ensure_delegates_unchecked_writes() {
    let output = U16Output::default();
    let mut output = BufferedOutput::ensure(output);

    // SAFETY: `input[1..3]` is a valid source range.
    let written = unsafe { output.write_unchecked(&[0, 1, 2], 1, 2) }
        .expect("wrapped output should support unchecked writes");

    assert_eq!(written, 2);

    let output = U16Output::default();
    let mut output = EnsuredBufferedOutput::AlreadyBuffered(output);

    // SAFETY: `input[0..2]` is a valid source range.
    let written = unsafe { output.write_unchecked(&[3, 4, 0], 0, 2) }
        .expect("already buffered output should support unchecked writes");

    assert_eq!(written, 2);
}

#[test]
fn test_buffered_output_ensure_delegates_unchecked_write_fully() {
    let output = U16Output::default();
    let mut output = BufferedOutput::ensure(output);

    // SAFETY: `input[1..3]` is a valid source range.
    unsafe { output.write_fully_unchecked(&[0, 1, 2], 1, 2) }
        .expect("wrapped output should support unchecked full writes");

    let output = U16Output::default();
    let mut output = EnsuredBufferedOutput::AlreadyBuffered(output);

    // SAFETY: `input[0..2]` is a valid source range.
    unsafe { output.write_fully_unchecked(&[3, 4, 0], 0, 2) }
        .expect("already buffered output should support unchecked full writes");
}

#[test]
fn test_buffered_output_ensure_delegates_seek() {
    let output = U16SeekOutput::new(vec![1, 2, 3, 4]);
    let mut output = BufferedOutput::ensure(output);
    let position = output
        .seek_to(SeekFrom::Start(2))
        .expect("wrapped seekable output should seek");

    assert_eq!(position, 2);

    let output = U16SeekOutput::new(vec![1, 2, 3, 4]);
    let mut output = EnsuredBufferedOutput::AlreadyBuffered(output);
    let position = output
        .seek_to(SeekFrom::Start(3))
        .expect("already buffered seekable output should seek");

    assert_eq!(position, 3);
}

#[test]
fn test_buffered_output_ensure_boxed_wraps_unbuffered_output() {
    let output = U16Output::default();
    let mut output = BufferedOutput::ensure_boxed(output);

    assert!(output.is_buffered());
    output
        .write_fully(&[1, 2, 3])
        .expect("boxed ensured output should write fully");
    output.flush().expect("boxed ensured output should flush");
}

#[test]
fn test_buffered_output_ensure_boxed_keeps_buffered_output() {
    let output = BufferedOutput::new(U16Output::default());
    let mut output = BufferedOutput::ensure_boxed(output);

    assert!(output.is_buffered());
    output
        .write_fully(&[1, 2])
        .expect("boxed already buffered output should write fully");
    output
        .flush()
        .expect("boxed already buffered output should flush");
}

#[derive(Clone)]
struct SharedWriter {
    output: Rc<RefCell<Vec<u8>>>,
}

impl SharedWriter {
    fn new(output: Rc<RefCell<Vec<u8>>>) -> Self {
        Self { output }
    }
}

impl Write for SharedWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.output.borrow_mut().extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct SharedFlushWriter {
    output: Rc<RefCell<Vec<u8>>>,
    flushes: Rc<Cell<usize>>,
}

impl SharedFlushWriter {
    fn new(output: Rc<RefCell<Vec<u8>>>, flushes: Rc<Cell<usize>>) -> Self {
        Self { output, flushes }
    }
}

impl Write for SharedFlushWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.output.borrow_mut().extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushes.set(self.flushes.get() + 1);
        Ok(())
    }
}

#[test]
fn test_buffered_output_writes_generic_items() {
    let inner = U16Output::default();
    let mut output = BufferedOutput::with_capacity(inner, 4);

    // SAFETY: The full input range is valid.
    let written = unsafe {
        output
            .write_unchecked(&[1, 2, 3], 0, 3)
            .expect("small write should be buffered")
    };
    assert_eq!(3, written);
    assert!(output.inner().values.is_empty());

    output
        .ensure_spare_capacity(2)
        .expect("spare request should flush pending items");
    assert_eq!(&[1, 2, 3], output.inner().values.as_slice());

    output.flush().expect("flush should reach inner output");
    assert!(output.inner().flushed);
}

#[test]
fn test_buffered_output_implements_output_for_generic_items() {
    let inner = U16Output::default();
    let mut output = BufferedOutput::with_capacity(inner, 4);
    let output: &mut dyn Output<Item = u16> = &mut output;

    // SAFETY: The full input range is valid.
    let written = unsafe {
        output
            .write_unchecked(&[1, 2, 3], 0, 3)
            .expect("buffered output should implement Output")
    };

    assert_eq!(3, written);
    output
        .flush()
        .expect("buffered output should flush pending items");
}

#[test]
fn test_buffered_output_trait_write_fully_unchecked_for_generic_items() {
    let inner = U16Output::default();
    let mut output = BufferedOutput::with_capacity(inner, 2);

    // SAFETY: `input[1..4]` is a valid source range.
    unsafe {
        <BufferedOutput<U16Output> as Output>::write_fully_unchecked(
            &mut output,
            &[0, 1, 2, 3],
            1,
            3,
        )
        .expect("trait write_fully_unchecked should write through buffer");
    }
    output
        .flush()
        .expect("buffered output should flush pending items");

    assert_eq!(&[1, 2, 3], output.inner().values.as_slice());
}

#[test]
fn test_buffered_output_trait_write_fully_for_generic_items() {
    let inner = U16Output::default();
    let mut output = BufferedOutput::with_capacity(inner, 2);

    <BufferedOutput<U16Output> as Output>::write_fully(&mut output, &[1, 2, 3])
        .expect("trait write_fully should write through buffer");
    output
        .flush()
        .expect("buffered output should flush pending items");

    assert_eq!(&[1, 2, 3], output.inner().values.as_slice());
}

#[test]
fn test_buffered_output_adapts_std_write_as_u8_output() {
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    // SAFETY: The source range is valid.
    unsafe {
        output
            .write_fully_unchecked(b"abc", 0, 3)
            .expect("std writer should be an Output<Item = u8>");
    }
    output.flush().expect("flush should succeed");

    assert_eq!(b"abc", output.inner().get_ref().as_slice());
}

#[test]
fn test_buffered_output_rejects_overreported_item_count() {
    let mut output = BufferedOutput::with_capacity(OverreportingOutput, 4);

    // SAFETY: The source range is valid.
    let error = unsafe {
        output
            .write_fully_unchecked(&[1, 2, 3, 4], 0, 4)
            .expect_err("overreported direct write count should fail")
    };

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "writer reported 5 items for a 4-item buffer",
        error.to_string()
    );
}

#[test]
fn test_output_u8_blanket_impl_propagates_std_flush_errors() {
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
            Ok(input.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush failed"))
        }
    }

    let mut writer = FailingWriter;

    let error = Output::flush(&mut writer)
        .expect_err("std flush error should be propagated");

    assert_eq!(ErrorKind::Other, error.kind());
}
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

fn flush_into_inner<W>(mut output: BufferedOutput<W>) -> W
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

fn write_spare_bytes<W>(output: &mut BufferedOutput<W>, bytes: &[u8])
where
    W: Write,
{
    let (buffer, index, count) = output.spare_raw_parts_mut();
    assert!(
        bytes.len() <= count,
        "test fixture writes beyond spare output buffer",
    );
    buffer[index..index + bytes.len()].copy_from_slice(bytes);
}

#[test]
fn test_new_and_inner_mut_expose_writer() {
    let mut output = BufferedOutput::new(Cursor::new(Vec::new()));

    Write::write_all(output.inner_mut(), b"raw")
        .expect("inner writer should be mutable");
    let cursor = flush_into_inner(output);

    assert_eq!(b"raw", cursor.into_inner().as_slice());
}

#[test]
fn test_capacity_returns_internal_buffer_capacity() {
    let output =
        BufferedOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);

    assert_eq!(4, output.capacity());
}

#[test]
fn test_spare_raw_parts_mut_and_advance_append_to_buffer() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");
    write_spare_bytes(&mut output, b"cd");
    // SAFETY: Two bytes were initialized in the spare range.
    unsafe {
        output.advance(2);
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_advance_marks_spare_bytes_as_written() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    write_spare_bytes(&mut output, b"ab");
    // SAFETY: Two bytes were initialized in the spare buffer, and the spare
    // capacity is four bytes.
    unsafe {
        output.advance(2);
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"ab", cursor.into_inner().as_slice());
}

#[test]
fn test_spare_raw_parts_mut_exposes_backing_buffer_index_and_count() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let (buffer, index, count) = output.spare_raw_parts_mut();
    assert_eq!(2, index);
    assert_eq!(2, count);
    buffer[index..index + count].copy_from_slice(b"cd");
    // SAFETY: Two bytes were initialized in the spare range reported by
    // `spare_raw_parts_mut`.
    unsafe {
        output.advance(count);
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_ensure_spare_capacity_rejects_oversized_request() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    let error = output
        .ensure_spare_capacity(5)
        .expect_err("oversized spare request should fail");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_ensure_spare_capacity_succeeds_without_flushing_when_space_remains() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"a")
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
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"abc")
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
    let mut output = BufferedOutput::with_capacity(writer, 4);
    output
        .write_fully(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .ensure_spare_capacity(2)
        .expect_err("flush error should be returned");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_write_fully_delegates_large_empty_buffer_write() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"abcd")
        .expect("large write should be delegated");
    let cursor = flush_into_inner(output);

    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_write_fully_delegated_large_write_retries_interrupted_writer() {
    let writer =
        ScriptedWriter::new(vec![WriteStep::Interrupted, WriteStep::Accept(4)]);
    let mut output = BufferedOutput::with_capacity(writer, 4);

    output
        .write_fully(b"abcd")
        .expect("interrupted delegated write_fully should be retried");

    assert_eq!(b"abcd", output.inner().output.as_slice());
}

#[test]
fn test_write_fully_delegated_large_write_returns_write_zero() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedOutput::with_capacity(writer, 4);

    let error = output
        .write_fully(b"abcd")
        .expect_err("zero-length delegated write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_write_fully_delegated_large_write_returns_writer_error() {
    let writer = ScriptedWriter::new(vec![WriteStep::Error(
        ErrorKind::BrokenPipe,
        "write failed",
    )]);
    let mut output = BufferedOutput::with_capacity(writer, 4);

    let error = output
        .write_fully(b"abcd")
        .expect_err("delegated writer error should be returned");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_indexed_write_fully_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"abc")
        .expect("buffered prefix should be accepted");

    output
        .write_fully(b"xy")
        .expect("small write should flush prefix and then buffer");
    assert_eq!(b"abc", output.inner().get_ref().as_slice());

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcxy", cursor.into_inner().as_slice());
}

#[test]
fn test_write_fully_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    // SAFETY: Both full input slices are valid source ranges.
    unsafe {
        output
            .write_fully_unchecked(b"abc", 0, 3)
            .expect("buffered prefix should be accepted");
        output
            .write_fully_unchecked(b"xy", 0, 2)
            .expect("small write should flush prefix and then buffer");
    }
    assert_eq!(b"abc", output.inner().get_ref().as_slice());

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcxy", cursor.into_inner().as_slice());
}

#[test]
fn test_write_delegates_large_empty_buffer_write() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    let count = output
        .write(b"abcd")
        .expect("large raw write should be delegated");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.inner().get_ref().as_slice());
}

#[test]
fn test_indexed_write_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
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
fn test_write_flushes_then_buffers_smaller_input() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    // SAFETY: Both full input slices are valid source ranges.
    let prefix_count = unsafe {
        output
            .write_unchecked(b"abc", 0, 3)
            .expect("buffered prefix should be accepted")
    };
    assert_eq!(3, prefix_count);

    // SAFETY: The full input slice is a valid source range.
    let count = unsafe {
        output
            .write_unchecked(b"xy", 0, 2)
            .expect("small raw write should flush prefix and then buffer")
    };
    assert_eq!(2, count);
    assert_eq!(b"abc", output.inner().get_ref().as_slice());

    let cursor = flush_into_inner(output);
    assert_eq!(b"abcxy", cursor.into_inner().as_slice());
}

#[test]
fn test_flush_buffer_accepts_empty_buffer() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .ensure_spare_capacity(1)
        .expect("empty spare reservation should complete without writing");

    assert!(output.inner().get_ref().is_empty());
}

#[test]
fn test_flush_buffer_retries_interrupted_writes() {
    let writer =
        ScriptedWriter::new(vec![WriteStep::Interrupted, WriteStep::Accept(4)]);
    let mut output = BufferedOutput::with_capacity(writer, 4);
    output
        .write_fully(b"abc")
        .expect("buffered write should succeed");

    output
        .ensure_spare_capacity(4)
        .expect("interrupted write should be retried");

    assert_eq!(b"abc", output.inner().output.as_slice());
}

#[test]
fn test_flush_buffer_returns_write_zero() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedOutput::with_capacity(writer, 4);
    output
        .write_fully(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .ensure_spare_capacity(4)
        .expect_err("zero-length write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_flush_buffer_preserves_unwritten_suffix_after_error() {
    let writer = ScriptedWriter::new(vec![
        WriteStep::Accept(2),
        WriteStep::Error(ErrorKind::BrokenPipe, "write failed"),
    ]);
    let mut output = BufferedOutput::with_capacity(writer, 8);
    output
        .write_fully(b"abcd")
        .expect("buffered write should succeed");

    let error = output
        .ensure_spare_capacity(8)
        .expect_err("writer error should be returned");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!(b"ab", output.inner().output.as_slice());

    output.inner_mut().steps.push_back(WriteStep::Accept(2));
    output
        .ensure_spare_capacity(8)
        .expect("remaining suffix should flush on retry");

    assert_eq!(b"abcd", output.inner().output.as_slice());
}

#[test]
fn test_flush_returns_inner_flush_error() {
    let writer = ScriptedWriter::with_flush_error();
    let mut output = BufferedOutput::with_capacity(writer, 4);

    let error = output
        .flush()
        .expect_err("inner flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("flush failed", error.to_string());
}

#[test]
fn test_write_fully_accepts_full_input_slice() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"abcd")
        .expect("write_fully should accept the full input slice");
    let cursor = flush_into_inner(output);

    assert_eq!(b"abcd", cursor.into_inner().as_slice());
}

#[test]
fn test_output_flush_delegates_to_buffered_flush() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"abc")
        .expect("buffered write should succeed");
    output.flush().expect("flush should drain buffer");

    assert_eq!(b"abc", output.inner().get_ref().as_slice());
}

#[test]
fn test_stream_position_reports_logical_position_without_flushing() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let position = output
        .stream_position()
        .expect("stream_position should report logical position");

    assert_eq!(2, position);
    assert!(
        output.inner().get_ref().is_empty(),
        "querying stream_position must not flush pending bytes",
    );
}

#[test]
fn test_seek_current_zero_reports_logical_position_without_flushing() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let position = output
        .seek_to(SeekFrom::Current(0))
        .expect("current-zero seek should report logical position");

    assert_eq!(2, position);
    assert!(
        output.inner().get_ref().is_empty(),
        "seek(Current(0)) must not flush pending bytes",
    );
}

#[test]
fn test_seek_flushes_pending_bytes_before_seeking() {
    let cursor = Cursor::new(vec![0; 4]);
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let position = output
        .seek_to(SeekFrom::Start(0))
        .expect("seek should flush pending bytes first");
    output
        .write_fully(b"xy")
        .expect("second write should be buffered");
    let cursor = flush_into_inner(output);

    assert_eq!(0, position);
    assert_eq!(b"xy\0\0", cursor.into_inner().as_slice());
}

#[test]
fn test_drop_flushes_pending_bytes_best_effort() {
    let captured = Rc::new(RefCell::new(Vec::new()));

    {
        let writer = SharedWriter::new(Rc::clone(&captured));
        let mut output = BufferedOutput::with_capacity(writer, 4);
        output
            .write_fully(b"ab")
            .expect("buffered write should succeed");
    }

    assert_eq!(b"ab", captured.borrow().as_slice());
}

#[test]
fn test_drop_drains_pending_bytes_without_flushing_inner_writer() {
    let captured = Rc::new(RefCell::new(Vec::new()));
    let flushes = Rc::new(Cell::new(0));

    {
        let writer =
            SharedFlushWriter::new(Rc::clone(&captured), Rc::clone(&flushes));
        let mut output = BufferedOutput::with_capacity(writer, 4);
        output
            .write_fully(b"ab")
            .expect("buffered write should succeed");
    }

    assert_eq!(b"ab", captured.borrow().as_slice());
    assert_eq!(
        0,
        flushes.get(),
        "drop should drain pending buffer without calling the wrapped flush",
    );
}

#[test]
fn test_seekable_items_flushes_pending_items_before_seeking() {
    let inner = U16SeekOutput::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(inner, 4);

    // SAFETY: The source range is valid for u16 items.
    unsafe {
        output
            .write_fully_unchecked(&[10, 20, 30], 0, 3)
            .expect("buffered write should keep items until flush")
    };

    let position = output
        .seek_to(SeekFrom::Start(3))
        .expect("item seek should flush pending items before seeking");
    let (inner, pending) = output.into_parts();

    assert_eq!(3, position);
    assert_eq!(vec![10, 20, 30], inner.values);
    assert_eq!(3, inner.position);
    assert!(pending.is_empty(), "seek should flush all pending items");
}

#[test]
fn test_seekable_items_supports_current_offset() {
    let mut output =
        BufferedOutput::with_capacity(U16SeekOutput::new(Vec::new()), 4);

    // SAFETY: The source range is valid for u16 items.
    unsafe {
        output
            .write_fully_unchecked(&[11, 12], 0, 2)
            .expect("buffered write should keep items")
    };

    let position = output
        .seek_to(SeekFrom::Current(1))
        .expect("current seek should be interpreted in u16 items");
    let (inner, pending) = output.into_parts();

    assert_eq!(3, position);
    assert_eq!(3, inner.position);
    assert_eq!(vec![11, 12, 0], inner.values);
    assert!(pending.is_empty(), "seek should flush all pending items");
}

#[test]
fn test_write_forwards_through_buffered_output() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    let accepted = output.write(b"abc").expect("write should succeed");
    assert_eq!(3, accepted);

    output.flush().expect("flush should succeed");
    assert_eq!(b"abc", output.inner().get_ref().as_slice());
}

#[test]
fn test_into_parts_returns_inner_and_pending_bytes_without_flushing() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let (cursor, pending) = output.into_parts();

    assert!(cursor.into_inner().is_empty());
    assert_eq!(b"ab", pending.readable());
}

#[test]
fn test_into_inner_flushes_pending_bytes_and_returns_inner() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    output
        .write_fully(b"ab")
        .expect("buffered write should succeed");

    let cursor = output
        .into_inner()
        .expect("into_inner should flush pending bytes");

    assert_eq!(b"ab", cursor.into_inner().as_slice());
}

#[test]
fn test_flush_error_keeps_output_owned_by_caller() {
    let writer = ScriptedWriter::new(vec![WriteStep::Zero]);
    let mut output = BufferedOutput::with_capacity(writer, 4);
    output
        .write_fully(b"abc")
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
    let mut output = BufferedOutput::with_capacity(writer, 4);
    output
        .write_fully(b"abc")
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
    let mut output = BufferedOutput::with_capacity(OverreportingWriter, 4);
    output
        .write_fully(b"abc")
        .expect("buffered write should succeed");

    let error = output
        .ensure_spare_capacity(4)
        .expect_err("overreported flush count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_write_rejects_invalid_delegated_write_count() {
    let mut output = BufferedOutput::with_capacity(OverreportingWriter, 4);

    let error = output
        .write(b"abcd")
        .expect_err("overreported delegated write count should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_output_trait_write_via_dyn_output() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    let output: &mut dyn Output<Item = u8> = &mut output;

    let written = output
        .write(b"abc")
        .expect("BufferedOutput should implement Output::write");

    assert_eq!(3, written);
    output.flush().expect("flush should succeed");
}

#[test]
fn test_buffered_output_trait_write_unchecked_via_dyn_output() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);
    let output: &mut dyn Output<Item = u8> = &mut output;

    // SAFETY: `b"bc"` is a valid source range inside `b"abc"`.
    let written = unsafe {
        output
            .write_unchecked(b"abc", 1, 2)
            .expect("BufferedOutput should implement Output::write_unchecked")
    };

    assert_eq!(2, written);
    output.flush().expect("flush should succeed");
}

#[test]
fn test_buffered_output_with_zero_capacity_uses_one() {
    let cursor = Cursor::new(Vec::new());
    let output = BufferedOutput::with_capacity(cursor, 0);

    assert_eq!(1, output.capacity());
}

#[test]
fn test_buffered_output_write_rejects_overreported_count_via_trait() {
    let mut output = BufferedOutput::with_capacity(OverreportingOutput, 4);
    let output: &mut dyn Output<Item = u16> = &mut output;

    let error = output
        .write(&[1, 2, 3, 4])
        .expect_err("trait write should validate reported counts");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_buffered_output_into_inner_flushes_generic_items() {
    let mut output = BufferedOutput::with_capacity(U16Output::default(), 4);

    // SAFETY: The source range is valid for u16 items.
    unsafe {
        output
            .write_fully_unchecked(&[1, 2], 0, 2)
            .expect("buffered write should succeed");
    }

    let inner = output
        .into_inner()
        .expect("into_inner should flush pending generic items");

    assert_eq!(&[1, 2], inner.values.as_slice());
    assert!(inner.flushed);
}

#[test]
fn test_write_cold_flushes_before_delegated_large_write() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"abc")
        .expect("small write should stay buffered");

    // SAFETY: The source range is valid and triggers the cold write path.
    let written = unsafe {
        output
            .write_unchecked(b"1234", 0, 4)
            .expect("large write should flush then delegate")
    };

    assert_eq!(4, written);
    let cursor = flush_into_inner(output);
    assert_eq!(b"abc1234", cursor.into_inner().as_slice());
}

#[test]
fn test_write_fully_cold_flushes_before_delegated_large_write_fully() {
    let cursor = Cursor::new(Vec::new());
    let mut output = BufferedOutput::with_capacity(cursor, 4);

    output
        .write_fully(b"abc")
        .expect("small write should stay buffered");

    // SAFETY: The source range is valid and triggers the cold write-all path.
    unsafe {
        output
            .write_fully_unchecked(b"1234", 0, 4)
            .expect("large write_fully should flush then delegate");
    }

    let cursor = flush_into_inner(output);
    assert_eq!(b"abc1234", cursor.into_inner().as_slice());
}

#[derive(Default)]
struct OverflowPositionOutput {
    values: Vec<u16>,
    position: u64,
}

impl Output for OverflowPositionOutput {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        self.values.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl qubit_io::Seekable for OverflowPositionOutput {
    type Unit = u16;

    fn seek_to(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Current(0) => Ok(self.position),
            SeekFrom::Start(offset) => {
                self.position = offset;
                Ok(offset)
            }
            _ => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

#[test]
fn test_stream_position_rejects_pending_item_overflow() {
    let mut output = BufferedOutput::with_capacity(
        OverflowPositionOutput {
            values: Vec::new(),
            position: u64::MAX,
        },
        4,
    );

    // SAFETY: The source range is valid for u16 items.
    unsafe {
        output
            .write_fully_unchecked(&[1, 2], 0, 2)
            .expect("buffered write should succeed");
    }

    let error = output
        .stream_position()
        .expect_err("pending items must not overflow logical position");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "buffered pending items overflow wrapped output position",
        error.to_string()
    );
}

#[test]
fn test_seekable_trait_object_dispatches_to_buffered_output() {
    let mut output =
        BufferedOutput::with_capacity(U16SeekOutput::new(Vec::new()), 4);
    let seekable: &mut dyn Seekable<Unit = u16> = &mut output;

    let position = seekable
        .seek_to(SeekFrom::Start(0))
        .expect("Seekable trait object should dispatch to buffered output");

    assert_eq!(0, position);
}
