// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::RefCell;
use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::Hasher;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use qubit_io::AsyncChecksumInput;
use qubit_io::AsyncChecksumOutput;
use qubit_io::AsyncClose;
use qubit_io::AsyncCountingInput;
use qubit_io::AsyncCountingOutput;
use qubit_io::AsyncInput;
use qubit_io::AsyncLimitInput;
use qubit_io::AsyncLimitOutput;
use qubit_io::AsyncOutput;

enum ReadStep {
    Data(Vec<u8>),
    Error(ErrorKind),
    Pending,
    Eof,
}

struct ScriptedInput {
    steps: VecDeque<ReadStep>,
    buffered: bool,
    marker: usize,
}

impl ScriptedInput {
    fn new(steps: impl IntoIterator<Item = ReadStep>, buffered: bool) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            buffered,
            marker: 0,
        }
    }
}

impl AsyncInput for ScriptedInput {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        self.buffered
    }

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        match self.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(bytes) => {
                let read = count.min(bytes.len());
                output[index..index + read].copy_from_slice(&bytes[..read]);
                if read < bytes.len() {
                    self.steps.push_front(ReadStep::Data(bytes[read..].to_vec()));
                }
                Poll::Ready(Ok(read))
            }
            ReadStep::Error(kind) => Poll::Ready(Err(Error::new(kind, "scripted read failure"))),
            ReadStep::Pending => Poll::Pending,
            ReadStep::Eof => Poll::Ready(Ok(0)),
        }
    }
}

enum WriteStep {
    Accept(usize),
    Error(ErrorKind),
    Pending,
}

enum FlushStep {
    Ready,
    Error(ErrorKind),
    Pending,
}

struct ScriptedOutput {
    bytes: Vec<u8>,
    write_steps: VecDeque<WriteStep>,
    flush_steps: VecDeque<FlushStep>,
    buffered: bool,
    marker: usize,
    closed: bool,
    close_error: Option<ErrorKind>,
}

impl ScriptedOutput {
    fn new(
        write_steps: impl IntoIterator<Item = WriteStep>,
        flush_steps: impl IntoIterator<Item = FlushStep>,
        buffered: bool,
    ) -> Self {
        Self {
            bytes: Vec::new(),
            write_steps: write_steps.into_iter().collect(),
            flush_steps: flush_steps.into_iter().collect(),
            buffered,
            marker: 0,
            closed: false,
            close_error: None,
        }
    }

    fn with_close_error(mut self, kind: ErrorKind) -> Self {
        self.close_error = Some(kind);
        self
    }
}

impl AsyncClose for ScriptedOutput {
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(kind) = self.close_error.take() {
            return Poll::Ready(Err(Error::new(kind, "scripted close failure")));
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

impl AsyncOutput for ScriptedOutput {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        self.buffered
    }

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        match self.write_steps.pop_front().unwrap_or(WriteStep::Accept(count)) {
            WriteStep::Accept(maximum) => {
                let written = count.min(maximum);
                self.bytes.extend_from_slice(&input[index..index + written]);
                Poll::Ready(Ok(written))
            }
            WriteStep::Error(kind) => Poll::Ready(Err(Error::new(kind, "scripted write failure"))),
            WriteStep::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.flush_steps.pop_front().unwrap_or(FlushStep::Ready) {
            FlushStep::Ready => Poll::Ready(Ok(())),
            FlushStep::Error(kind) => Poll::Ready(Err(Error::new(kind, "scripted flush failure"))),
            FlushStep::Pending => Poll::Pending,
        }
    }
}

#[test]
fn test_async_output_wrappers_propagate_close() {
    let inner = ScriptedOutput::new([], [], false);
    let output = AsyncLimitOutput::new(inner, 10);
    let output = AsyncChecksumOutput::new(output, DefaultHasher::new());
    let mut output = AsyncCountingOutput::new(output);
    let mut cx = Context::from_waker(Waker::noop());

    AsyncClose::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close should complete")
        .expect("close should succeed");

    let output = output.into_inner();
    let (output, _) = output.into_parts();
    let inner = output.into_inner();
    assert!(inner.closed);
}

#[test]
fn test_async_output_wrappers_reject_forbidden_flush_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut counting = AsyncCountingOutput::new(ScriptedOutput::new([], [FlushStep::Error(kind)], false));
        let mut limit = AsyncLimitOutput::new(ScriptedOutput::new([], [FlushStep::Error(kind)], false), 1);
        let mut checksum = AsyncChecksumOutput::new(
            ScriptedOutput::new([], [FlushStep::Error(kind)], false),
            DefaultHasher::new(),
        );
        let mut cx = Context::from_waker(Waker::noop());

        for error in [
            Pin::new(&mut counting)
                .poll_flush(&mut cx)
                .expect_ready("counting flush error should be ready")
                .expect_err("counting flush should reject a forbidden error"),
            Pin::new(&mut limit)
                .poll_flush(&mut cx)
                .expect_ready("limit flush error should be ready")
                .expect_err("limit flush should reject a forbidden error"),
            Pin::new(&mut checksum)
                .poll_flush(&mut cx)
                .expect_ready("checksum flush error should be ready")
                .expect_err("checksum flush should reject a forbidden error"),
        ] {
            assert_eq!(ErrorKind::InvalidData, error.kind());
        }
    }
}

#[test]
fn test_async_output_wrappers_reject_forbidden_close_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut counting = AsyncCountingOutput::new(ScriptedOutput::new([], [], false).with_close_error(kind));
        let mut limit = AsyncLimitOutput::new(ScriptedOutput::new([], [], false).with_close_error(kind), 1);
        let mut checksum = AsyncChecksumOutput::new(
            ScriptedOutput::new([], [], false).with_close_error(kind),
            DefaultHasher::new(),
        );
        let mut cx = Context::from_waker(Waker::noop());

        for error in [
            AsyncClose::poll_close(Pin::new(&mut counting), &mut cx)
                .expect_ready("counting close error should be ready")
                .expect_err("counting close should reject a forbidden error"),
            AsyncClose::poll_close(Pin::new(&mut limit), &mut cx)
                .expect_ready("limit close error should be ready")
                .expect_err("limit close should reject a forbidden error"),
            AsyncClose::poll_close(Pin::new(&mut checksum), &mut cx)
                .expect_ready("checksum close error should be ready")
                .expect_err("checksum close should reject a forbidden error"),
        ] {
            assert_eq!(ErrorKind::InvalidData, error.kind());
        }
    }
}

struct ByteInput {
    bytes: Vec<u8>,
    position: usize,
    pending: bool,
}

impl AsyncInput for ByteInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let available = self.bytes.len().saturating_sub(self.position);
        let read = available.min(count).min(2);
        output[index..index + read].copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

struct ByteOutput {
    bytes: Arc<Mutex<Vec<u8>>>,
    pending: bool,
}

impl AsyncOutput for ByteOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let written = count.min(2);
        self.bytes
            .lock()
            .expect("lock should succeed")
            .extend_from_slice(&input[index..index + written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn complete<F>(future: F) -> F::Output
where
    F: Future,
{
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    for _ in 0..256 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
    panic!("test future did not complete");
}

#[test]
fn test_async_input_wrappers_limit_count_and_hash_successful_reads() -> io::Result<()> {
    let input = ByteInput {
        bytes: b"abcdef".to_vec(),
        position: 0,
        pending: true,
    };
    let input = AsyncLimitInput::new(input, 4);
    let input = AsyncChecksumInput::new(input, DefaultHasher::new());
    let mut input = AsyncCountingInput::new(input);
    let mut output = [0_u8; 8];

    assert_eq!(4, complete(input.read_fully_async(&mut output))?);
    assert_eq!(b"abcd", &output[..4]);
    assert_eq!(4, input.items_read());
    assert_eq!(0, input.inner().inner().remaining());

    let mut expected = DefaultHasher::new();
    expected.write(b"abcd");
    assert_eq!(expected.finish(), input.inner().checksum());
    Ok(())
}

#[test]
fn test_async_output_wrappers_limit_count_and_hash_successful_writes() -> io::Result<()> {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let output = ByteOutput {
        bytes: bytes.clone(),
        pending: true,
    };
    let output = AsyncLimitOutput::new(output, 4);
    let output = AsyncChecksumOutput::new(output, DefaultHasher::new());
    let mut output = AsyncCountingOutput::new(output);

    complete(output.write_fully_async(b"abcd"))?;
    assert_eq!(0, complete(output.write_async(b"ef"))?);
    complete(output.flush_async())?;
    assert_eq!(b"abcd", bytes.lock().expect("lock should succeed").as_slice());
    assert_eq!(4, output.items_written());
    assert_eq!(0, output.inner().inner().remaining());

    let mut expected = DefaultHasher::new();
    expected.write(b"abcd");
    assert_eq!(expected.finish(), output.inner().checksum());
    Ok(())
}

#[test]
fn test_async_counting_input_counts_only_successful_reads_and_exposes_inner() {
    let inner = ScriptedInput::new(
        [
            ReadStep::Pending,
            ReadStep::Error(ErrorKind::PermissionDenied),
            ReadStep::Data(vec![1, 2]),
        ],
        true,
    );
    let mut input = AsyncCountingInput::new(inner);
    assert!(input.is_buffered());
    assert_eq!(0, input.items_read());
    assert_eq!(0, input.bytes_read());
    assert_eq!(0, input.inner().marker);
    input.inner_mut().marker = 1;

    let mut cx = Context::from_waker(Waker::noop());
    let mut output = [0_u8; 3];
    assert!(Pin::new(&mut input).poll_read(&mut cx, &mut output).is_pending());
    let error = Pin::new(&mut input)
        .poll_read(&mut cx, &mut output)
        .expect_ready("counting read error should be ready")
        .expect_err("counting read should fail");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!(0, input.items_read());
    assert_eq!(
        2,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut output)
            .expect_ready("counting read should be ready")
            .expect("counting read should succeed")
    );
    assert_eq!(2, input.items_read());
    assert_eq!(2, input.bytes_read());
    assert_eq!([1, 2, 0], output);
    assert_eq!(1, input.into_inner().marker);
}

#[test]
fn test_async_counting_output_counts_only_successful_writes_and_delegates_flush() {
    let inner = ScriptedOutput::new(
        [
            WriteStep::Pending,
            WriteStep::Error(ErrorKind::BrokenPipe),
            WriteStep::Accept(2),
        ],
        [FlushStep::Pending, FlushStep::Ready],
        true,
    );
    let mut output = AsyncCountingOutput::new(inner);
    assert!(output.is_buffered());
    assert_eq!(0, output.items_written());
    assert_eq!(0, output.bytes_written());
    assert_eq!(0, output.inner().marker);
    output.inner_mut().marker = 1;

    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut output).poll_write(&mut cx, b"abc").is_pending());
    let error = Pin::new(&mut output)
        .poll_write(&mut cx, b"abc")
        .expect_ready("counting write error should be ready")
        .expect_err("counting write should fail");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());
    assert_eq!(0, output.items_written());
    assert_eq!(
        2,
        Pin::new(&mut output)
            .poll_write(&mut cx, b"abc")
            .expect_ready("counting write should be ready")
            .expect("counting write should succeed")
    );
    assert_eq!(2, output.items_written());
    assert_eq!(2, output.bytes_written());
    assert!(Pin::new(&mut output).poll_flush(&mut cx).is_pending());
    Pin::new(&mut output)
        .poll_flush(&mut cx)
        .expect_ready("counting flush should be ready")
        .expect("counting flush should succeed");

    let inner = output.into_inner();
    assert_eq!(1, inner.marker);
    assert_eq!(b"ab", inner.bytes.as_slice());
}

#[test]
fn test_async_limit_input_bounds_reads_and_preserves_state_on_pending_or_error() {
    let inner = ScriptedInput::new(
        [
            ReadStep::Pending,
            ReadStep::Error(ErrorKind::TimedOut),
            ReadStep::Data(vec![1, 2, 3]),
        ],
        true,
    );
    let mut input = AsyncLimitInput::new(inner, 2);
    assert!(input.is_buffered());
    assert_eq!(2, input.remaining());
    assert_eq!(0, input.inner().marker);
    input.inner_mut().marker = 1;

    let mut cx = Context::from_waker(Waker::noop());
    let mut empty = [];
    assert_eq!(
        0,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut empty)
            .expect_ready("empty limited read should be ready")
            .expect("empty limited read should succeed")
    );
    let mut output = [0_u8; 4];
    assert!(Pin::new(&mut input).poll_read(&mut cx, &mut output).is_pending());
    assert_eq!(2, input.remaining());
    let error = Pin::new(&mut input)
        .poll_read(&mut cx, &mut output)
        .expect_ready("limited read error should be ready")
        .expect_err("limited read should fail");
    assert_eq!(ErrorKind::TimedOut, error.kind());
    assert_eq!(2, input.remaining());
    assert_eq!(
        2,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut output)
            .expect_ready("limited read should be ready")
            .expect("limited read should succeed")
    );
    assert_eq!(0, input.remaining());
    assert!(matches!(input.inner().steps.front(), Some(ReadStep::Data(bytes)) if bytes == &[3]));
    assert_eq!(
        0,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut output)
            .expect_ready("exhausted limited read should be ready")
            .expect("exhausted limited read should succeed")
    );
    let mut inner = input.into_inner();
    assert_eq!(1, inner.marker);
    assert_eq!(
        1,
        Pin::new(&mut inner)
            .poll_read(&mut cx, &mut output)
            .expect_ready("inner should return the retained suffix")
            .expect("inner read should succeed")
    );
    assert_eq!([3, 2, 0, 0], output);
    assert_eq!(
        0,
        Pin::new(&mut inner)
            .poll_read(&mut cx, &mut output)
            .expect_ready("inner should reach EOF after the suffix")
            .expect("inner EOF read should succeed")
    );
}

#[test]
fn test_scripted_input_retains_data_suffix_across_small_reads() {
    let mut input = ScriptedInput::new([ReadStep::Data(vec![1, 2, 3]), ReadStep::Eof], false);
    let mut cx = Context::from_waker(Waker::noop());
    let mut first = [0_u8; 1];
    let mut rest = [0_u8; 2];
    assert_eq!(
        1,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut first)
            .expect_ready("first byte should be ready")
            .expect("first read should succeed")
    );
    assert_eq!(
        2,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut rest)
            .expect_ready("suffix should be ready")
            .expect("suffix read should succeed")
    );
    assert_eq!([1, 2, 3], [first[0], rest[0], rest[1]]);
    assert_eq!(
        0,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut rest)
            .expect_ready("EOF should be ready")
            .expect("EOF should succeed")
    );
}

#[test]
fn test_scripted_input_zero_count_does_not_consume_scripted_events() {
    for steps in [
        vec![ReadStep::Data(vec![7])],
        vec![ReadStep::Pending],
        vec![ReadStep::Error(ErrorKind::Other)],
    ] {
        let mut input = ScriptedInput::new(steps, false);
        let mut cx = Context::from_waker(Waker::noop());
        let mut output = [0_u8; 1];
        assert_eq!(
            0,
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut cx, &mut output, 0, 0) }
                .expect_ready("zero count should be ready")
                .expect("zero count should succeed")
        );
        match Pin::new(&mut input).poll_read(&mut cx, &mut output) {
            Poll::Ready(Ok(1)) if output[0] == 7 => {}
            Poll::Pending => {}
            Poll::Ready(Err(error)) if error.kind() == ErrorKind::Other => {}
            other => panic!("scripted event was consumed or changed: {other:?}"),
        }
    }
}

#[test]
fn test_async_limit_output_bounds_writes_and_preserves_state_on_pending_or_error() {
    let inner = ScriptedOutput::new(
        [
            WriteStep::Pending,
            WriteStep::Error(ErrorKind::WouldBlock),
            WriteStep::Accept(2),
        ],
        [FlushStep::Error(ErrorKind::Other)],
        true,
    );
    let mut output = AsyncLimitOutput::new(inner, 2);
    assert!(output.is_buffered());
    assert_eq!(2, output.remaining());
    assert_eq!(0, output.inner().marker);
    output.inner_mut().marker = 1;

    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(
        0,
        Pin::new(&mut output)
            .poll_write(&mut cx, &[])
            .expect_ready("empty limited write should be ready")
            .expect("empty limited write should succeed")
    );
    assert!(Pin::new(&mut output).poll_write(&mut cx, b"abcd").is_pending());
    assert_eq!(2, output.remaining());
    let error = Pin::new(&mut output)
        .poll_write(&mut cx, b"abcd")
        .expect_ready("limited write error should be ready")
        .expect_err("limited write should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(2, output.remaining());
    assert_eq!(
        2,
        Pin::new(&mut output)
            .poll_write(&mut cx, b"abcd")
            .expect_ready("limited write should be ready")
            .expect("limited write should succeed")
    );
    assert_eq!(0, output.remaining());
    assert_eq!(
        0,
        Pin::new(&mut output)
            .poll_write(&mut cx, b"z")
            .expect_ready("exhausted limited write should be ready")
            .expect("exhausted limited write should succeed")
    );
    let error = Pin::new(&mut output)
        .poll_flush(&mut cx)
        .expect_ready("limited flush error should be ready")
        .expect_err("limited flush should fail");
    assert_eq!(ErrorKind::Other, error.kind());

    let inner = output.into_inner();
    assert_eq!(1, inner.marker);
    assert_eq!(b"ab", inner.bytes.as_slice());
}

#[test]
fn test_async_checksum_input_hashes_only_successful_reads_and_exposes_parts() {
    let inner = ScriptedInput::new(
        [
            ReadStep::Pending,
            ReadStep::Error(ErrorKind::InvalidInput),
            ReadStep::Data(b"ab".to_vec()),
        ],
        true,
    );
    let mut input = AsyncChecksumInput::new(inner, DefaultHasher::new());
    assert!(input.is_buffered());
    assert_eq!(0, input.inner().marker);
    input.inner_mut().marker = 1;
    input.hasher_mut().write(b"x");
    let initial = input.hasher().finish();
    assert_eq!(initial, input.checksum());

    let mut cx = Context::from_waker(Waker::noop());
    let mut output = [0_u8; 3];
    assert!(Pin::new(&mut input).poll_read(&mut cx, &mut output).is_pending());
    assert_eq!(initial, input.checksum());
    let error = Pin::new(&mut input)
        .poll_read(&mut cx, &mut output)
        .expect_ready("checksum read error should be ready")
        .expect_err("checksum read should fail");
    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(initial, input.checksum());
    assert_eq!(
        2,
        Pin::new(&mut input)
            .poll_read(&mut cx, &mut output)
            .expect_ready("checksum read should be ready")
            .expect("checksum read should succeed")
    );
    let mut expected = DefaultHasher::new();
    expected.write(b"x");
    expected.write(b"ab");
    assert_eq!(expected.finish(), input.checksum());

    let (inner, hasher) = input.into_parts();
    assert_eq!(1, inner.marker);
    assert_eq!(expected.finish(), hasher.finish());
}

#[test]
fn test_async_checksum_output_hashes_only_successful_writes_and_exposes_parts() {
    let inner = ScriptedOutput::new(
        [
            WriteStep::Pending,
            WriteStep::Error(ErrorKind::NotConnected),
            WriteStep::Accept(2),
        ],
        [FlushStep::Pending, FlushStep::Ready],
        true,
    );
    let mut output = AsyncChecksumOutput::new(inner, DefaultHasher::new());
    assert!(output.is_buffered());
    assert_eq!(0, output.inner().marker);
    output.inner_mut().marker = 1;
    output.hasher_mut().write(b"x");
    let initial = output.hasher().finish();
    assert_eq!(initial, output.checksum());

    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut output).poll_write(&mut cx, b"abc").is_pending());
    assert_eq!(initial, output.checksum());
    let error = Pin::new(&mut output)
        .poll_write(&mut cx, b"abc")
        .expect_ready("checksum write error should be ready")
        .expect_err("checksum write should fail");
    assert_eq!(ErrorKind::NotConnected, error.kind());
    assert_eq!(initial, output.checksum());
    assert_eq!(
        2,
        Pin::new(&mut output)
            .poll_write(&mut cx, b"abc")
            .expect_ready("checksum write should be ready")
            .expect("checksum write should succeed")
    );
    let mut expected = DefaultHasher::new();
    expected.write(b"x");
    expected.write(b"ab");
    assert_eq!(expected.finish(), output.checksum());
    assert!(Pin::new(&mut output).poll_flush(&mut cx).is_pending());
    Pin::new(&mut output)
        .poll_flush(&mut cx)
        .expect_ready("checksum flush should be ready")
        .expect("checksum flush should succeed");

    let (inner, hasher) = output.into_parts();
    assert_eq!(1, inner.marker);
    assert_eq!(b"ab", inner.bytes.as_slice());
    assert_eq!(expected.finish(), hasher.finish());
}

trait PollResultExt<T> {
    fn expect_ready(self, message: &str) -> T;
}

impl<T> PollResultExt<T> for Poll<T> {
    fn expect_ready(self, message: &str) -> T {
        match self {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("{message}"),
        }
    }
}

#[derive(Clone, Copy)]
enum ProbeStage {
    Read = 0,
    Write = 1,
    Flush = 2,
    Close = 3,
}

struct ProbeState {
    address: Option<*const AddressCheckedStream>,
    read_position: usize,
    received: Vec<u8>,
    closed: bool,
    calls: [usize; 4],
    ready: [bool; 4],
    wakers: [Option<Waker>; 4],
}

struct AddressCheckedStream {
    state: Rc<RefCell<ProbeState>>,
    _pin: PhantomPinned,
}

impl AddressCheckedStream {
    fn new() -> (Self, Rc<RefCell<ProbeState>>) {
        let state = Rc::new(RefCell::new(ProbeState {
            address: None,
            read_position: 0,
            received: Vec::new(),
            closed: false,
            calls: [0; 4],
            ready: [false; 4],
            wakers: std::array::from_fn(|_| None),
        }));
        (
            Self {
                state: state.clone(),
                _pin: PhantomPinned,
            },
            state,
        )
    }

    fn poll_stage(self: Pin<&Self>, cx: &Context<'_>, stage: ProbeStage) -> bool {
        let this = self.get_ref();
        let address = this as *const Self;
        let mut state = this.state.borrow_mut();
        if let Some(expected) = state.address {
            assert_eq!(expected, address, "pinned inner stream moved");
        } else {
            state.address = Some(address);
        }
        let index = stage as usize;
        state.calls[index] += 1;
        if state.ready[index] {
            true
        } else {
            state.wakers[index] = Some(cx.waker().clone());
            false
        }
    }
}

impl AsyncInput for AddressCheckedStream {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if !self.as_ref().poll_stage(cx, ProbeStage::Read) {
            return Poll::Pending;
        }
        let this = self.as_ref().get_ref();
        let mut state = this.state.borrow_mut();
        let available = 4usize.saturating_sub(state.read_position);
        let read = available.min(count).min(2);
        let start = state.read_position;
        for (offset, byte) in b"abcd"[start..start + read].iter().enumerate() {
            output[index + offset] = *byte;
        }
        state.read_position += read;
        Poll::Ready(Ok(read))
    }
}

impl AsyncOutput for AddressCheckedStream {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if !self.as_ref().poll_stage(cx, ProbeStage::Write) {
            return Poll::Pending;
        }
        let this = self.as_ref().get_ref();
        let mut state = this.state.borrow_mut();
        let written = count.min(2);
        state.received.extend_from_slice(&input[index..index + written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.as_ref().poll_stage(cx, ProbeStage::Flush) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

impl AsyncClose for AddressCheckedStream {
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.as_ref().poll_stage(cx, ProbeStage::Close) {
            self.as_ref().get_ref().state.borrow_mut().closed = true;
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

#[derive(Default)]
struct PinRecordingHasher(Vec<Vec<u8>>);

impl Hasher for PinRecordingHasher {
    fn finish(&self) -> u64 {
        self.0.iter().flatten().fold(0u64, |sum, byte| sum + u64::from(*byte))
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.push(bytes.to_vec());
    }
}

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn notify_probe(state: &Rc<RefCell<ProbeState>>, stage: ProbeStage, counter: &WakeCounter) {
    let index = stage as usize;
    let waker = {
        let mut state = state.borrow_mut();
        state.ready[index] = true;
        state.wakers[index].take().expect("pending probe must save a waker")
    };
    let previous = counter.0.load(Ordering::SeqCst);
    waker.wake();
    assert_eq!(previous + 1, counter.0.load(Ordering::SeqCst));
}

fn move_pin_box<T>(value: Pin<Box<T>>) -> Pin<Box<T>> {
    value
}

#[test]
fn test_async_pinned_input_wrappers_keep_inner_address() {
    let (stream, state) = AddressCheckedStream::new();
    let wrapped = AsyncCountingInput::new(AsyncChecksumInput::new(
        AsyncLimitInput::new(stream, 4),
        PinRecordingHasher::default(),
    ));
    let mut input = Box::pin(wrapped);
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let mut buffer = [0xEE; 4];
    assert!(AsyncInput::poll_read(input.as_mut(), &mut cx, &mut buffer).is_pending());
    assert_eq!([0xEE; 4], buffer);
    assert_eq!(0, input.as_ref().get_ref().items_read());
    assert_eq!(4, input.as_ref().get_ref().inner().inner().remaining());
    assert!(input.as_ref().get_ref().inner().hasher().0.is_empty());
    notify_probe(&state, ProbeStage::Read, &counter);
    assert_eq!(
        2,
        AsyncInput::poll_read(input.as_mut(), &mut cx, &mut buffer)
            .expect_ready("first read ready")
            .expect("read succeeds")
    );
    assert_eq!(b"ab", &buffer[..2]);
    assert_eq!([b'a', b'b', 0xEE, 0xEE], buffer);
    assert_eq!(2, input.as_ref().get_ref().items_read());
    assert_eq!(2, input.as_ref().get_ref().inner().inner().remaining());
    assert_eq!(vec![b"ab".to_vec()], input.as_ref().get_ref().inner().hasher().0);
    let mut input = move_pin_box(input);
    assert_eq!(
        2,
        AsyncInput::poll_read(input.as_mut(), &mut cx, &mut buffer)
            .expect_ready("second read ready")
            .expect("read succeeds")
    );
    assert_eq!(b"cd", &buffer[..2]);
    assert_eq!(4, input.as_ref().get_ref().items_read());
    assert_eq!(0, input.as_ref().get_ref().inner().inner().remaining());
    assert_eq!(
        vec![b"ab".to_vec(), b"cd".to_vec()],
        input.as_ref().get_ref().inner().hasher().0
    );
    let calls = state.borrow().calls[ProbeStage::Read as usize];
    assert_eq!(
        0,
        AsyncInput::poll_read(input.as_mut(), &mut cx, &mut buffer)
            .expect_ready("exhausted read ready")
            .expect("read succeeds")
    );
    assert_eq!(calls, state.borrow().calls[ProbeStage::Read as usize]);
    assert!(state.borrow().address.is_some());
}

#[test]
fn test_async_pinned_output_wrappers_keep_inner_address() {
    let (stream, state) = AddressCheckedStream::new();
    let wrapped = AsyncCountingOutput::new(AsyncChecksumOutput::new(
        AsyncLimitOutput::new(stream, 4),
        PinRecordingHasher::default(),
    ));
    let mut output = Box::pin(wrapped);
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    assert!(AsyncOutput::poll_write(output.as_mut(), &mut cx, b"abcde").is_pending());
    assert_eq!(0, output.as_ref().get_ref().items_written());
    assert_eq!(4, output.as_ref().get_ref().inner().inner().remaining());
    assert!(state.borrow().received.is_empty());
    assert!(output.as_ref().get_ref().inner().hasher().0.is_empty());
    notify_probe(&state, ProbeStage::Write, &counter);
    assert_eq!(
        2,
        AsyncOutput::poll_write(output.as_mut(), &mut cx, b"abcde")
            .expect_ready("write ready")
            .expect("write succeeds")
    );
    assert_eq!(2, output.as_ref().get_ref().items_written());
    assert_eq!(2, output.as_ref().get_ref().inner().inner().remaining());
    assert_eq!(
        2,
        AsyncOutput::poll_write(output.as_mut(), &mut cx, b"cde")
            .expect_ready("write ready")
            .expect("write succeeds")
    );
    assert_eq!(4, output.as_ref().get_ref().items_written());
    assert_eq!(0, output.as_ref().get_ref().inner().inner().remaining());
    assert_eq!(b"abcd".to_vec(), state.borrow().received);
    assert_eq!(
        vec![b"ab".to_vec(), b"cd".to_vec()],
        output.as_ref().get_ref().inner().hasher().0
    );
    let calls = state.borrow().calls[ProbeStage::Write as usize];
    assert_eq!(
        0,
        AsyncOutput::poll_write(output.as_mut(), &mut cx, b"e")
            .expect_ready("exhausted write ready")
            .expect("write succeeds")
    );
    assert_eq!(calls, state.borrow().calls[ProbeStage::Write as usize]);
    assert_eq!(4, output.as_ref().get_ref().items_written());

    assert!(AsyncOutput::poll_flush(output.as_mut(), &mut cx).is_pending());
    notify_probe(&state, ProbeStage::Flush, &counter);
    AsyncOutput::poll_flush(output.as_mut(), &mut cx)
        .expect_ready("flush ready")
        .expect("flush succeeds");
    assert!(AsyncClose::poll_close(output.as_mut(), &mut cx).is_pending());
    assert!(!state.borrow().closed);
    notify_probe(&state, ProbeStage::Close, &counter);
    AsyncClose::poll_close(output.as_mut(), &mut cx)
        .expect_ready("close ready")
        .expect("close succeeds");
    assert_eq!(2, state.borrow().calls[ProbeStage::Flush as usize]);
    assert_eq!(2, state.borrow().calls[ProbeStage::Close as usize]);
    assert!(state.borrow().closed);
    assert_eq!(4, output.as_ref().get_ref().items_written());
    assert_eq!(
        vec![b"ab".to_vec(), b"cd".to_vec()],
        output.as_ref().get_ref().inner().hasher().0
    );
    assert_eq!(b"abcd".to_vec(), state.borrow().received);
    assert!(state.borrow().address.is_some());
}
