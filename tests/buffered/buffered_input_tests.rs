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

use qubit_io::{
    BufferedInput,
    EnsuredBufferedInput,
    Input,
    Seekable,
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

struct SpecializedReadFullyInput {
    data: Vec<u16>,
    position: usize,
    read_fully_unchecked_calls: usize,
}

impl SpecializedReadFullyInput {
    fn new(data: Vec<u16>) -> Self {
        Self {
            data,
            position: 0,
            read_fully_unchecked_calls: 0,
        }
    }
}

impl Input for SpecializedReadFullyInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        if self.position >= self.data.len() {
            return Ok(0);
        }
        let read = count.min(1).min(self.data.len() - self.position);
        output[index..index + read]
            .copy_from_slice(&self.data[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }

    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        self.read_fully_unchecked_calls += 1;
        let read = count.min(self.data.len() - self.position);
        output[index..index + read]
            .copy_from_slice(&self.data[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

enum ReadFullyStep {
    Data(Vec<u16>),
    Interrupted,
    Error(ErrorKind, &'static str),
    Overreport,
}

struct ScriptedReadFullyInput {
    steps: VecDeque<ReadFullyStep>,
    read_fully_unchecked_calls: usize,
}

impl ScriptedReadFullyInput {
    fn new(steps: Vec<ReadFullyStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
            read_fully_unchecked_calls: 0,
        }
    }
}

impl Input for ScriptedReadFullyInput {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u16],
        _index: usize,
        _count: usize,
    ) -> std::io::Result<usize> {
        panic!("read_unchecked should not be used for large direct reads")
    }

    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        self.read_fully_unchecked_calls += 1;
        match self.steps.pop_front() {
            Some(ReadFullyStep::Data(data)) => {
                let read = count.min(data.len());
                output[index..index + read].copy_from_slice(&data[..read]);
                Ok(read)
            }
            Some(ReadFullyStep::Interrupted) => {
                Err(Error::new(ErrorKind::Interrupted, "interrupted"))
            }
            Some(ReadFullyStep::Error(kind, message)) => {
                Err(Error::new(kind, message))
            }
            Some(ReadFullyStep::Overreport) => Ok(count + 1),
            None => Ok(0),
        }
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
fn test_buffered_input_reads_generic_items() {
    let inner = U16Input::new(vec![vec![1, 2, 3], vec![4, 5]]);
    let mut input = BufferedInput::with_capacity(inner, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    assert_eq!(&[1, 2, 3], unread_units(&input).as_slice());
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    assert!(input.fill_until(4).expect("refill should append items"));
    assert_eq!(&[2, 3, 4, 5], unread_units(&input).as_slice());

    let mut output = [0_u16; 3];
    // SAFETY: `output[0..3]` is a valid destination range.
    let read = unsafe {
        input
            .read_unchecked(&mut output, 0, 3)
            .expect("buffered read should succeed")
    };

    assert_eq!(3, read);
    assert_eq!([2, 3, 4], output);
    assert_eq!(&[5], unread_units(&input).as_slice());
}

#[test]
fn test_buffered_input_reports_buffered() {
    let input = BufferedInput::with_capacity(U16Input::new(vec![]), 4);

    assert!(input.is_buffered());
}

#[test]
fn test_buffered_input_ensure_wraps_unbuffered_input() {
    let input = U16Input::new(vec![vec![1, 2, 3]]);
    let mut input = BufferedInput::ensure(input);

    assert!(matches!(input, EnsuredBufferedInput::Buffered(_)));
    assert!(input.is_buffered());

    let mut output = [0; 2];
    let read = input
        .read(&mut output)
        .expect("ensured buffered input should read items");

    assert_eq!(read, 2);
    assert_eq!(output, [1, 2]);
}

#[test]
fn test_buffered_input_ensure_keeps_buffered_input() {
    let input = U16Input::new(vec![vec![1, 2, 3]]);
    let input = BufferedInput::new(input);
    let mut input = BufferedInput::ensure(input);

    assert!(matches!(input, EnsuredBufferedInput::AlreadyBuffered(_)));
    assert!(input.is_buffered());

    let mut output = [0; 3];
    let read = input
        .read_fully(&mut output)
        .expect("already buffered input should still read fully");

    assert_eq!(read, 3);
    assert_eq!(output, [1, 2, 3]);
}

#[test]
fn test_buffered_input_ensure_delegates_safe_reads_for_both_branches() {
    let input = U16Input::new(vec![vec![1, 2]]);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 2];
    let read = input
        .read_fully(&mut output)
        .expect("wrapped input should support safe full reads");

    assert_eq!(read, 2);
    assert_eq!(output, [1, 2]);

    let input = U16Input::new(vec![vec![3, 4]]);
    let input = BufferedInput::new(input);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 2];
    let read = input
        .read(&mut output)
        .expect("already buffered input should support safe reads");

    assert_eq!(read, 2);
    assert_eq!(output, [3, 4]);
}

#[test]
fn test_buffered_input_ensure_delegates_unchecked_reads() {
    let input = U16Input::new(vec![vec![1, 2, 3]]);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 3];

    // SAFETY: `output[1..3]` is a valid destination range.
    let read = unsafe { input.read_unchecked(&mut output, 1, 2) }
        .expect("wrapped input should support unchecked reads");

    assert_eq!(read, 2);
    assert_eq!(output, [0, 1, 2]);

    let input = U16Input::new(vec![vec![4, 5, 6]]);
    let input = BufferedInput::new(input);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 3];

    // SAFETY: `output[0..2]` is a valid destination range.
    let read = unsafe { input.read_unchecked(&mut output, 0, 2) }
        .expect("already buffered input should support unchecked reads");

    assert_eq!(read, 2);
    assert_eq!(output, [4, 5, 0]);
}

#[test]
fn test_buffered_input_ensure_delegates_unchecked_read_fully() {
    let input = U16Input::new(vec![vec![1], vec![2, 3]]);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 3];

    // SAFETY: `output[0..3]` is a valid destination range.
    let read = unsafe { input.read_fully_unchecked(&mut output, 0, 3) }
        .expect("wrapped input should support unchecked full reads");

    assert_eq!(read, 3);
    assert_eq!(output, [1, 2, 3]);

    let input = U16Input::new(vec![vec![4], vec![5, 6]]);
    let input = BufferedInput::new(input);
    let mut input = BufferedInput::ensure(input);
    let mut output = [0; 3];

    // SAFETY: `output[0..3]` is a valid destination range.
    let read = unsafe { input.read_fully_unchecked(&mut output, 0, 3) }
        .expect("already buffered input should support unchecked full reads");

    assert_eq!(read, 3);
    assert_eq!(output, [4, 5, 6]);
}

#[test]
fn test_buffered_input_ensure_delegates_seek() {
    let input = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::ensure(input);
    let position = input
        .seek_to(SeekFrom::Start(3))
        .expect("wrapped seekable input should seek");

    assert_eq!(position, 3);

    let input = Cursor::new(b"abcdef".to_vec());
    let input = BufferedInput::new(input);
    let mut input = BufferedInput::ensure(input);
    let position = input
        .seek_to(SeekFrom::Start(4))
        .expect("already buffered seekable input should seek");

    assert_eq!(position, 4);
}

#[test]
fn test_buffered_input_ensure_boxed_wraps_unbuffered_input() {
    let input = U16Input::new(vec![vec![1, 2, 3]]);
    let mut input = BufferedInput::ensure_boxed(input);

    assert!(input.is_buffered());

    let mut output = [0; 3];
    let read = input
        .read_fully(&mut output)
        .expect("boxed ensured input should read fully");

    assert_eq!(read, 3);
    assert_eq!(output, [1, 2, 3]);
}

#[test]
fn test_buffered_input_ensure_boxed_keeps_buffered_input() {
    let input = BufferedInput::new(U16Input::new(vec![vec![1, 2]]));
    let mut input = BufferedInput::ensure_boxed(input);

    assert!(input.is_buffered());

    let mut output = [0; 2];
    let read = input
        .read(&mut output)
        .expect("boxed already buffered input should read");

    assert_eq!(read, 2);
    assert_eq!(output, [1, 2]);
}

#[test]
fn test_buffered_input_ensure_boxed_dyn_wraps_unbuffered_input() {
    let input: Box<dyn Input<Item = u16>> =
        Box::new(U16Input::new(vec![vec![1, 2, 3]]));
    let mut input = BufferedInput::<U16Input>::ensure_boxed_dyn(input);

    assert!(input.is_buffered());

    let mut output = [0; 3];
    let read = input
        .read_fully(&mut output)
        .expect("boxed dyn ensured input should read fully");

    assert_eq!(read, 3);
    assert_eq!(output, [1, 2, 3]);
}

#[test]
fn test_buffered_input_ensure_boxed_dyn_keeps_buffered_input() {
    let input: Box<dyn Input<Item = u16>> =
        Box::new(BufferedInput::new(U16Input::new(vec![vec![1, 2]])));
    let mut input = BufferedInput::<U16Input>::ensure_boxed_dyn(input);

    assert!(input.is_buffered());

    let mut output = [0; 2];
    let read = input
        .read(&mut output)
        .expect("boxed dyn already buffered input should read");

    assert_eq!(read, 2);
    assert_eq!(output, [1, 2]);
}

#[test]
fn test_buffered_input_implements_input_for_generic_items() {
    let inner = U16Input::new(vec![vec![1, 2, 3]]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let input: &mut dyn Input<Item = u16> = &mut input;
    let mut output = [0_u16; 2];

    // SAFETY: `output[0..2]` is a valid destination range.
    let read = unsafe {
        input
            .read_unchecked(&mut output, 0, 2)
            .expect("buffered input should implement Input")
    };

    assert_eq!(2, read);
    assert_eq!([1, 2], output);
}

#[test]
fn test_buffered_input_trait_read_fully_unchecked_for_generic_items() {
    let inner = U16Input::new(vec![vec![1], vec![2, 3]]);
    let mut input = BufferedInput::with_capacity(inner, 2);
    let mut output = [0_u16; 4];

    // SAFETY: `output[1..4]` is a valid destination range.
    let read = unsafe {
        <BufferedInput<U16Input> as Input>::read_fully_unchecked(
            &mut input,
            &mut output,
            1,
            3,
        )
        .expect("trait read_fully_unchecked should fill through buffer")
    };

    assert_eq!(3, read);
    assert_eq!([0, 1, 2, 3], output);
}

#[test]
fn test_buffered_input_trait_read_fully_for_generic_items() {
    let inner = U16Input::new(vec![vec![1, 2], vec![3]]);
    let mut input = BufferedInput::with_capacity(inner, 2);
    let mut output = [0_u16; 4];

    let read =
        <BufferedInput<U16Input> as Input>::read_fully(&mut input, &mut output)
            .expect("trait read_fully should return partial count at EOF");

    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0], output);
}

#[test]
fn test_buffered_input_read_fully_empty_output_does_not_read() {
    let mut input = BufferedInput::with_capacity(PanicOnRead, 4);
    let mut output = [];

    let read = input
        .read_fully(&mut output)
        .expect("empty read_fully should not read inner");

    assert_eq!(0, read);
}

#[test]
fn test_buffered_input_read_fully_small_empty_buffer_returns_zero_at_eof() {
    let reader = ScriptedReader::new(vec![ReadStep::Eof]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    let mut output = [0_u8; 3];

    let read = input
        .read_fully(&mut output)
        .expect("small read_fully should return zero at EOF");

    assert_eq!(0, read);
    assert_eq!([0, 0, 0], output);
}

#[test]
fn test_buffered_input_read_fully_serves_satisfied_range_from_unread() {
    let inner = U16Input::new(vec![vec![1, 2, 3, 4], vec![5]]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let mut output = [0_u16; 2];

    assert!(input.fill_more().expect("initial refill should succeed"));

    let read = input
        .read_fully(&mut output)
        .expect("buffered unread bytes should satisfy read fully");

    assert_eq!(2, read);
    assert_eq!([1, 2], output);
    assert_eq!(&[3, 4], unread_units(&input).as_slice());
}

#[test]
fn test_buffered_input_read_fully_delegates_large_remainder_to_inner() {
    let inner = SpecializedReadFullyInput::new(vec![1, 2, 3, 4, 5, 6]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let mut output = [0_u16; 5];

    assert!(input.fill_more().expect("initial refill should succeed"));

    // SAFETY: `output[..5]` is a valid destination range.
    let read = unsafe {
        input
            .read_fully_unchecked(&mut output, 0, 5)
            .expect("large remaining read should delegate to inner")
    };

    assert_eq!(5, read);
    assert_eq!([1, 2, 3, 4, 5], output);
    assert_eq!(0, input.unread_len());
    assert_eq!(1, input.inner().read_fully_unchecked_calls);
}

#[test]
fn test_buffered_input_read_fully_direct_inner_retries_interrupted() {
    let inner = ScriptedReadFullyInput::new(vec![
        ReadFullyStep::Interrupted,
        ReadFullyStep::Data(vec![1, 2, 3, 4]),
    ]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let mut output = [0_u16; 4];

    let read = input
        .read_fully(&mut output)
        .expect("direct inner read_fully should retry interrupted errors");

    assert_eq!(4, read);
    assert_eq!([1, 2, 3, 4], output);
    assert_eq!(2, input.inner().read_fully_unchecked_calls);
}

#[test]
fn test_buffered_input_read_fully_direct_inner_returns_error() {
    let inner = ScriptedReadFullyInput::new(vec![ReadFullyStep::Error(
        ErrorKind::PermissionDenied,
        "read failed",
    )]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let mut output = [0_u16; 4];

    let error = input
        .read_fully(&mut output)
        .expect_err("direct inner read_fully should return errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(1, input.inner().read_fully_unchecked_calls);
}

#[test]
fn test_buffered_input_read_fully_direct_inner_rejects_overreported_count() {
    let inner = ScriptedReadFullyInput::new(vec![ReadFullyStep::Overreport]);
    let mut input = BufferedInput::with_capacity(inner, 4);
    let mut output = [0_u16; 4];

    let error = input
        .read_fully(&mut output)
        .expect_err("direct inner read_fully should validate reported count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "reader reported 5 items for a 4-item buffer",
        error.to_string()
    );
    assert_eq!(1, input.inner().read_fully_unchecked_calls);
}

#[test]
fn test_buffered_input_read_fully_retries_interrupted_reads() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Interrupted,
        ReadStep::Data(b"abc".to_vec()),
    ]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    let mut output = [0_u8; 3];

    let read = input
        .read_fully(&mut output)
        .expect("read_fully should retry interrupted reads");

    assert_eq!(3, read);
    assert_eq!(b"abc", &output);
}

#[test]
fn test_buffered_input_read_fully_returns_non_interrupted_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "read failed",
    )]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    let mut output = [0_u8; 3];

    let error = input
        .read_fully(&mut output)
        .expect_err("read_fully should return non-interrupted errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_buffered_input_adapts_std_read_as_u8_input() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    let mut output = [0_u8; 5];

    // SAFETY: `output[1..4]` is a valid destination range.
    let read = unsafe {
        input
            .read_unchecked(&mut output, 1, 3)
            .expect("std reader should be an Input<Item = u8>")
    };

    assert_eq!(3, read);
    assert_eq!([0, b'a', b'b', b'c', 0], output);
}

#[test]
fn test_buffered_input_rejects_overreported_item_count() {
    let mut input = BufferedInput::with_capacity(OverreportingInput, 4);

    let error = input
        .fill_more()
        .expect_err("overreported read count should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "reader reported 5 items for a 4-item buffer",
        error.to_string()
    );
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

struct TrackingSeekReader {
    data: Vec<u8>,
    position: u64,
    seek_calls: usize,
}

impl TrackingSeekReader {
    fn new(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
            seek_calls: 0,
        }
    }
}

impl Read for TrackingSeekReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let position = usize::try_from(self.position).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "position exceeds usize")
        })?;
        if position >= self.data.len() {
            return Ok(0);
        }
        let count = (self.data.len() - position).min(output.len());
        output[..count].copy_from_slice(&self.data[position..position + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for TrackingSeekReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.seek_calls += 1;
        let current = i128::from(self.position);
        let end = i128::try_from(self.data.len()).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "stream length exceeds i128")
        })?;
        let target = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => current + i128::from(offset),
            SeekFrom::End(offset) => end + i128::from(offset),
        };
        let position = u64::try_from(target).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "seek target is negative")
        })?;
        self.position = position;
        Ok(self.position)
    }
}

struct InconsistentPositionReader {
    data: Vec<u8>,
    position: u64,
}

impl InconsistentPositionReader {
    fn new(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
        }
    }
}

impl Read for InconsistentPositionReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let position = usize::try_from(self.position).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "position exceeds usize")
        })?;
        if position >= self.data.len() {
            return Ok(0);
        }
        let read = (self.data.len() - position).min(output.len());
        output[..read].copy_from_slice(&self.data[position..position + read]);
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for InconsistentPositionReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let current = i128::from(self.position);
        let end = i128::try_from(self.data.len()).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "stream length exceeds i128")
        })?;
        let target = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => current + i128::from(offset),
            SeekFrom::End(offset) => end + i128::from(offset),
        };
        self.position = u64::try_from(target).map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "seek target is negative")
        })?;
        if let SeekFrom::Current(0) = position {
            Ok(0)
        } else {
            Ok(self.position)
        }
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

fn unread_units<I>(input: &BufferedInput<I>) -> Vec<I::Item>
where
    I: Input,
    I::Item: Copy + Default,
{
    let count = input.unread_len();
    let mut unread = vec![I::Item::default(); count];
    // SAFETY: `unread[..count]` is a valid destination range that does not
    // overlap with the buffered input storage.
    unsafe {
        input.copy_unread_to(&mut unread, 0, count);
    }
    unread
}

#[test]
fn test_new_and_accessors_expose_inner_reader() {
    let mut input = BufferedInput::new(Cursor::new(b"abc".to_vec()));

    assert_eq!(0, input.unread_len());
    assert_eq!(0, input.inner().position());

    input.inner_mut().set_position(2);
    let (cursor, unread) = input.into_parts();

    assert_eq!(2, cursor.position());
    assert!(unread.is_empty());
}

#[test]
fn test_into_inner_discards_unread_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested item is currently buffered.
    unsafe {
        input.consume(1);
    }

    let cursor = input.into_inner();

    assert_eq!(4, cursor.position());
}

#[test]
fn test_capacity_returns_internal_buffer_capacity() {
    let cursor = Cursor::new(b"abc".to_vec());
    let input = BufferedInput::with_capacity(cursor, 4);

    assert_eq!(4, input.capacity());
}

#[test]
fn test_unread_returns_current_buffered_window() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    assert_eq!(b"bcd", input.unread());
}

#[test]
fn test_consume_advances_without_bounds_check() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    // SAFETY: The buffer has four readable bytes, so consuming two is valid.
    unsafe {
        input.consume(2);
    }

    assert_eq!(b"cd", unread_units(&input).as_slice());
}

#[test]
fn test_into_parts_returns_inner_and_unread_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let (cursor, unread) = input.into_parts();

    assert_eq!(4, cursor.position());
    assert_eq!(b"bcd", unread.readable());
}

#[test]
fn test_copy_unread_to_copies_backing_buffer_window() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let mut unread = [0_u8; 5];
    // SAFETY: `unread[1..4]` is a valid destination range that does not
    // overlap with the buffered input storage.
    unsafe {
        input.copy_unread_to(&mut unread, 1, 3);
    }

    assert_eq!([0, b'b', b'c', b'd', 0], unread);
}

#[test]
fn test_fill_more_exposes_unread_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(
        input
            .fill_more()
            .expect("fill_more should refill from wrapped reader")
    );
    assert_eq!(b"abcd", input.unread());
    // SAFETY: The test has ensured two items are currently buffered.
    unsafe {
        input.consume(2);
    }
    assert_eq!(b"cd", input.unread());
    // SAFETY: The test has ensured two items are currently buffered.
    unsafe {
        input.consume(2);
    }
    assert!(
        input
            .fill_more()
            .expect("fill_more should refill after consumption")
    );
    assert_eq!(b"ef", input.unread());
}

#[test]
fn test_fill_more_preserves_unread_tail_and_appends_new_bytes() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    assert_eq!(b"abcd", unread_units(&input).as_slice());

    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(2);
    }
    assert_eq!(b"cd", unread_units(&input).as_slice());

    assert!(input.fill_more().expect("second refill should succeed"));
    assert_eq!(b"cdef", unread_units(&input).as_slice());
}

#[test]
fn test_fill_until_buffers_requested_available_bytes() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"abc".to_vec()),
        ReadStep::Data(b"de".to_vec()),
    ]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    assert!(
        input.fill_until(4).expect(
            "fill_until should read until requested bytes are buffered"
        )
    );

    assert_eq!(b"bcde", unread_units(&input).as_slice());
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

    assert_eq!(b"ab", unread_units(&input).as_slice());
}

#[test]
fn test_fill_until_returns_read_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "fill failed",
    )]);
    let mut input = BufferedInput::with_capacity(reader, 4);

    let error = input
        .fill_until(1)
        .expect_err("fill_until should return reader errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("fill failed", error.to_string());
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
    assert_eq!(0, input.unread_len());
}

#[test]
fn test_ensure_available_succeeds_when_requested_bytes_are_buffered() {
    let cursor = Cursor::new(b"abcd".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    input
        .ensure_available(3)
        .expect("ensure_available should succeed with enough bytes");

    assert_eq!(b"abcd", unread_units(&input).as_slice());
}

#[test]
fn test_fill_more_returns_false_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(!input.fill_more().expect("EOF refill should succeed"));
    assert_eq!(0, input.unread_len());
}

#[test]
fn test_fill_more_leaves_empty_unread_slice_at_eof() {
    let cursor = Cursor::new(Vec::new());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(!input.fill_more().expect("fill_more at EOF should succeed"));
    assert_eq!(b"", input.unread());
}

#[test]
fn test_fill_more_returns_refill_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "fill_buf failed",
    )]);
    let mut input = BufferedInput::with_capacity(reader, 4);

    let error = input
        .fill_more()
        .expect_err("fill_more should return refill errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("fill_buf failed", error.to_string());
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
    assert_eq!(b"ab", unread_units(&input).as_slice());
}

#[test]
fn test_fill_more_appends_when_tail_capacity_remains() {
    let reader = ScriptedReader::new(vec![
        ReadStep::Data(b"ab".to_vec()),
        ReadStep::Data(b"cd".to_vec()),
    ]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    assert!(input.fill_more().expect("second refill should append"));

    assert_eq!(b"bcd", unread_units(&input).as_slice());
}

#[test]
fn test_fill_more_rejects_refill_when_buffer_is_full() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    assert_eq!(4, input.unread_len());

    let error = input
        .fill_more()
        .expect_err("full buffer should require consumption before refill");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        "buffered input is full; consume buffered items before refilling",
        error.to_string()
    );
    assert_eq!(b"abcd", unread_units(&input).as_slice());
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
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let mut output = [0_u8; 3];
    let count = input
        .read(output.as_mut_slice())
        .expect("buffered read should succeed");

    assert_eq!(3, count);
    assert_eq!(b"bcd", &output);
}

#[test]
fn test_read_unchecked_writes_at_output_index_and_count() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }
    let mut output = [b'.'; 6];

    // SAFETY: `output[2..5]` is a valid writable range.
    let count = unsafe {
        input
            .read_unchecked(output.as_mut_slice(), 2, 3)
            .expect("indexed unchecked read should succeed")
    };

    assert_eq!(3, count);
    assert_eq!(b"..bcd.", &output);
    assert_eq!(0, input.unread_len());
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
fn test_read_delegated_large_empty_buffer_returns_reader_error() {
    let reader = ScriptedReader::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
        "delegated read failed",
    )]);
    let mut input = BufferedInput::with_capacity(reader, 4);
    let mut output = [0_u8; 4];

    let error = input
        .read(output.as_mut_slice())
        .expect_err("delegated reader error should be returned");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("delegated read failed", error.to_string());
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
    assert_eq!(b"cd", unread_units(&input).as_slice());
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
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let position = input
        .stream_position()
        .expect("seek should use logical position");

    assert_eq!(1, position);
}

#[test]
fn test_seek_current_within_buffer_preserves_prefetched_bytes() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }
    assert_eq!(b"bcd", unread_units(&input).as_slice());

    let position = input
        .seek_to(SeekFrom::Current(2))
        .expect("current seek within buffer should succeed");

    assert_eq!(3, position);
    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(b"d", unread_units(&input).as_slice());
}

#[test]
fn test_seek_current_large_offset_outside_buffer_delegates_to_inner_seek() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let large_offset = i64::from(u32::MAX) + 1;
    let position = input
        .seek_to(SeekFrom::Current(large_offset))
        .expect("large current seek outside buffer should delegate");

    assert_eq!(
        u64::try_from(large_offset).expect("offset is positive") + 1,
        position
    );
    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(0, input.unread_len());
}

#[test]
fn test_seek_current_within_buffer_uses_retained_window() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }
    assert_eq!(b"bcd", unread_units(&input).as_slice());

    input
        .seek_to(SeekFrom::Current(2))
        .expect("relative seek within buffer should succeed");

    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(b"d", unread_units(&input).as_slice());
}

#[test]
fn test_seek_current_within_buffer_rewinds_prefix() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(3);
    }

    input
        .seek_to(SeekFrom::Current(-2))
        .expect("negative seek inside buffer should rewind");

    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(b"bcd", unread_units(&input).as_slice());
}

#[test]
fn test_seek_relative_outside_buffer_delegates_to_underlying_seek() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }
    assert_eq!(b"bcd", unread_units(&input).as_slice());

    input
        .seek_to(SeekFrom::Current(6))
        .expect("seek beyond buffer should call underlying seek");

    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(7, input.inner().position);
    assert_eq!(0, input.unread_len());
}

#[test]
fn test_seekable_trait_object_dispatches_to_seek_impl() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let _ = <BufferedInput<TrackingSeekReader> as Seekable>::seek_to(
        &mut input,
        SeekFrom::Current(1),
    )
    .expect("seekable trait impl should be callable");

    assert!(input.unread_len() <= 3);
}

#[test]
fn test_seekable_trait_object_seek_from_start_and_end() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let position = <BufferedInput<TrackingSeekReader> as Seekable>::seek_to(
        &mut input,
        SeekFrom::Start(2),
    )
    .expect("trait seek start should call underlying source");

    assert_eq!(2, position);
    assert_eq!(0, input.unread_len());
    assert_eq!(1, input.inner().seek_calls);

    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let position = <BufferedInput<TrackingSeekReader> as Seekable>::seek_to(
        &mut input,
        SeekFrom::End(-1),
    )
    .expect("trait seek end should call underlying source");

    assert_eq!(5, position);
    assert_eq!(0, input.unread_len());
    assert_eq!(1, input.inner().seek_calls);
}

#[test]
fn test_seekable_ufcs_methods_cover_trait_impl() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }
    assert_eq!(b"bcd", unread_units(&input).as_slice());

    let position = Seekable::seek_to(&mut input, SeekFrom::Current(2))
        .expect("trait seek within buffer should use existing cache");
    assert_eq!(3, position);
    assert_eq!(1, input.inner().seek_calls);

    let position = Seekable::seek_to(&mut input, SeekFrom::Start(1))
        .expect("absolute trait seek should delegate to inner seek");
    assert_eq!(1, position);
    assert_eq!(2, input.inner().seek_calls);
    assert_eq!(0, input.unread_len());

    Seekable::seek_to(&mut input, SeekFrom::Current(6)).expect(
        "trait seek_relative outside buffer should delegate to inner seek",
    );
    assert_eq!(3, input.inner().seek_calls);
    assert_eq!(0, input.unread_len());

    let position = input
        .stream_position()
        .expect("stream_position should use buffered logical position");
    assert_eq!(7, position);
}

#[test]
fn test_seek_to_current_error_from_inner_seek() {
    let mut input = BufferedInput::with_capacity(FailingSeekReader, 4);

    let error = input
        .seek_to(SeekFrom::Current(1))
        .expect_err("underlying seek error should be surfaced");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("seek failed", error.to_string());
}

#[test]
fn test_ensure_available_u16_reader_consumes_partial_on_eof() {
    let mut input =
        BufferedInput::with_capacity(U16Input::new(vec![vec![42]]), 4);

    let error = input
        .ensure_available(2)
        .expect_err("ensure_available should fail when source ends early");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(0, input.unread_len());
}

#[test]
fn test_stream_position_preserves_prefetched_bytes() {
    let reader = TrackingSeekReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let position = input
        .stream_position()
        .expect("stream position should use logical position");

    assert_eq!(1, position);
    assert_eq!(1, input.inner().seek_calls);
    assert_eq!(b"bcd", unread_units(&input).as_slice());
}

#[test]
fn test_stream_position_errors_when_inner_reports_too_early_position() {
    let reader = InconsistentPositionReader::new(b"abcdef");
    let mut input = BufferedInput::with_capacity(reader, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: The test has ensured the requested items are currently buffered.
    unsafe {
        input.consume(1);
    }

    let error = input
        .stream_position()
        .expect_err("stream position should validate buffered state");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "buffered unread items exceed wrapped input position",
        error.to_string()
    );
}

#[test]
fn test_seek_rejects_current_offset_underflow() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let error = input
        .seek_to(SeekFrom::Current(i64::MIN))
        .expect_err("underflowing adjusted offset should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_seek_accepts_absolute_position_and_discards_buffer() {
    let cursor = Cursor::new(b"abcdef".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));

    let position = input
        .seek_to(SeekFrom::Start(3))
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
        .seek_to(SeekFrom::Start(0))
        .expect_err("underlying seek error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("seek failed", error.to_string());
}

#[test]
fn test_stream_position_returns_underlying_seek_error() {
    let mut input = BufferedInput::with_capacity(FailingSeekReader, 4);

    let error = input
        .stream_position()
        .expect_err("stream_position should propagate underlying seek errors");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("seek failed", error.to_string());
}

#[test]
fn test_ensure_available_propagates_fill_until_error() {
    let mut input = BufferedInput::new(Cursor::new(b"abc".to_vec()));

    let error = input
        .ensure_available(input.capacity() + 1)
        .expect_err("ensure_available should propagate fill_until errors");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
}

#[test]
fn test_buffered_input_trait_read_via_dyn_input() {
    let cursor = Cursor::new(b"abc".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);
    assert!(input.fill_more().expect("initial refill should succeed"));
    let input: &mut dyn Input<Item = u8> = &mut input;
    let mut output = [0_u8; 2];

    let read = input
        .read(&mut output)
        .expect("BufferedInput should implement Input::read");

    assert_eq!(2, read);
    assert_eq!(b"ab", &output);
}

#[test]
fn test_buffered_input_with_zero_capacity_uses_one() {
    let cursor = Cursor::new(b"a".to_vec());
    let input = BufferedInput::with_capacity(cursor, 0);

    assert_eq!(1, input.capacity());
}

#[test]
fn test_inner_returns_wrapped_input() {
    let cursor = Cursor::new(b"abc".to_vec());
    let input = BufferedInput::new(cursor);

    assert_eq!(0, input.inner().position());
}

#[test]
fn test_fill_more_backshifts_when_buffer_is_full_with_unread() {
    let cursor = Cursor::new(b"abcdefgh".to_vec());
    let mut input = BufferedInput::with_capacity(cursor, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    // SAFETY: One buffered item is consumed to leave unread data at the tail.
    unsafe {
        input.consume(1);
    }
    assert!(input.fill_more().expect("backshift refill should succeed"));
    assert_eq!(4, input.unread_len());
    assert_eq!(b"bcde", unread_units(&input).as_slice());
}

#[test]
fn test_u16_ensure_available_succeeds_when_items_are_buffered() {
    let inner = U16Input::new(vec![vec![1, 2, 3]]);
    let mut input = BufferedInput::with_capacity(inner, 4);

    assert!(input.fill_more().expect("initial refill should succeed"));
    input
        .ensure_available(2)
        .expect("ensure_available should succeed for generic item inputs");
    assert_eq!(&[1, 2, 3], unread_units(&input).as_slice());
}

#[test]
fn test_buffered_input_read_rejects_overreported_delegated_count() {
    let mut input = BufferedInput::with_capacity(OverreportingInput, 4);
    let mut output = [0_u16; 4];

    let error = input
        .read(&mut output)
        .expect_err("delegated reads should validate reported counts");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_ensure_available_succeeds_after_refill_from_empty_buffer() {
    let mut input = BufferedInput::new(Cursor::new(b"abc".to_vec()));

    input
        .ensure_available(2)
        .expect("ensure_available should refill an initially empty buffer");

    assert!(input.unread_len() >= 2);
    assert_eq!(b'a', unread_units(&input)[0]);
    assert_eq!(b'b', unread_units(&input)[1]);
}
