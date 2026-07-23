// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::future::Future;
use std::io::{
    Error,
    ErrorKind,
};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};
use std::task::{
    Context,
    Poll,
    Wake,
    Waker,
};

use qubit_io::{
    AsyncInput,
    ReadExactFuture,
    ReadFullyFuture,
};

enum ReadStep {
    Data(Vec<u8>),
    Error(ErrorKind),
    Pending,
    Eof,
}

struct ScriptedAsyncInput {
    steps: VecDeque<ReadStep>,
    poll_count: usize,
    registered_waker: Option<Waker>,
    _pinned: PhantomPinned,
}

impl ScriptedAsyncInput {
    fn new(steps: Vec<ReadStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
            poll_count: 0,
            registered_waker: None,
            _pinned: PhantomPinned,
        }
    }
}

impl AsyncInput for ScriptedAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: This implementation does not move the pinned value.
        let this = unsafe { self.get_unchecked_mut() };
        this.poll_count += 1;
        match this.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(data) => {
                let read = count.min(data.len());
                output[index..index + read].copy_from_slice(&data[..read]);
                Poll::Ready(Ok(read))
            }
            ReadStep::Error(kind) => {
                Poll::Ready(Err(Error::new(kind, "read failed")))
            }
            ReadStep::Pending => {
                this.registered_waker = Some(cx.waker().clone());
                Poll::Pending
            }
            ReadStep::Eof => Poll::Ready(Ok(0)),
        }
    }
}

struct OverreportingAsyncInput;

impl AsyncInput for OverreportingAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(count + 1))
    }
}

fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_async_input_is_object_safe_and_defaults_to_unbuffered() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![]));
    let input: Pin<&mut dyn AsyncInput<Item = u8>> = input.as_mut();

    assert!(!input.is_buffered());
}

#[test]
fn test_async_input_poll_read_rejects_overreported_count() {
    let mut input = OverreportingAsyncInput;
    let mut output = [0_u8; 3];
    let mut cx = context();

    let result =
        AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output);

    let error = result
        .expect_ready("overreported read should be ready")
        .expect_err("overreported read should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_async_input_zero_length_read_does_not_poll_inner() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![]));
    let mut output = [];
    let mut cx = context();

    let result = AsyncInput::poll_read(input.as_mut(), &mut cx, &mut output);

    assert!(matches!(result, Poll::Ready(Ok(0))));
    assert_eq!(0, input.poll_count);
}

#[test]
fn test_async_input_poll_read_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input =
            Box::pin(ScriptedAsyncInput::new(vec![ReadStep::Error(kind)]));
        let mut output = [0_u8; 1];
        let mut cx = context();

        let error = AsyncInput::poll_read(input.as_mut(), &mut cx, &mut output)
            .expect_ready("forbidden read error should be ready")
            .expect_err("forbidden read error should fail");

        assert_eq!(ErrorKind::InvalidData, error.kind());
        assert_eq!([0], output);
    }
}

#[test]
fn test_async_input_pending_registers_waker_without_progress() {
    struct TestWake(AtomicBool);

    impl Wake for TestWake {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    let mut input = Box::pin(ScriptedAsyncInput::new(vec![ReadStep::Pending]));
    let mut output = [0_u8; 1];
    let wake_state = Arc::new(TestWake(AtomicBool::new(false)));
    let waker = Waker::from(Arc::clone(&wake_state));
    let mut cx = Context::from_waker(&waker);

    let result = AsyncInput::poll_read(input.as_mut(), &mut cx, &mut output);

    assert!(result.is_pending());
    assert_eq!([0], output);
    let registered_waker = input
        .registered_waker
        .as_ref()
        .expect("pending read should register the current waker");
    registered_waker.wake_by_ref();
    assert!(wake_state.0.load(Ordering::Relaxed));
}

#[test]
fn test_read_async_preserves_pending_then_returns_partial_read() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![
        ReadStep::Pending,
        ReadStep::Data(vec![1, 2]),
    ]));
    let mut output = [0_u8; 4];
    let mut future = qubit_io::ReadFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("second poll should complete")
        .expect("read should succeed");
    assert_eq!(2, read);
    drop(future);
    assert_eq!([1, 2, 0, 0], output);
}

#[test]
fn test_read_fully_async_handles_pending_partial_and_eof() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![
        ReadStep::Data(vec![1, 2]),
        ReadStep::Pending,
        ReadStep::Data(vec![3]),
        ReadStep::Eof,
    ]));
    let mut output = [0_u8; 4];
    let mut future = ReadFullyFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    assert_eq!(2, future.items_read());
    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("second poll should complete")
        .expect("read_fully should succeed");
    assert_eq!(3, read);
    drop(future);
    assert_eq!([1, 2, 3, 0], output);
}

#[test]
fn test_read_exact_async_reports_unexpected_eof_with_progress() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![
        ReadStep::Data(vec![1, 2]),
        ReadStep::Eof,
    ]));
    let mut output = [0_u8; 3];
    let mut future = ReadExactFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    let error = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("EOF should complete")
        .expect_err("short exact read should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(2, future.items_read());
}

#[test]
fn test_read_exact_async_preserves_pending_and_input_errors() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![
        ReadStep::Pending,
        ReadStep::Error(ErrorKind::PermissionDenied),
    ]));
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    let error = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("input error should complete")
        .expect_err("input error should be preserved");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!(0, future.items_read());
}

#[test]
fn test_read_exact_async_empty_destination_and_repoll_contract() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![]));
    let mut output = [];
    let mut future = ReadExactFuture::new(input.as_mut(), &mut output);
    let mut cx = context();
    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Future::poll(Pin::new(&mut future), &mut cx);
    }));
    assert!(result.is_err());
}

#[test]
fn test_read_exactly_async_default_method_fills_destination() {
    struct Input;

    impl AsyncInput for Input {
        type Item = u8;

        unsafe fn poll_read_unchecked(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            output: &mut [u8],
            index: usize,
            count: usize,
        ) -> Poll<std::io::Result<usize>> {
            output[index..index + count].fill(7);
            Poll::Ready(Ok(count))
        }
    }

    let mut input = Input;
    let mut output = [0_u8; 2];
    let mut future = input.read_exactly_async(&mut output);
    let mut cx = context();

    Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("exact read should complete")
        .expect("exact read should succeed");
    drop(future);
    assert_eq!([7, 7], output);
}

#[test]
fn test_read_fully_async_default_method_reads_unpin_input() {
    struct OneShotInput(bool);

    impl AsyncInput for OneShotInput {
        type Item = u8;

        unsafe fn poll_read_unchecked(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            output: &mut [u8],
            index: usize,
            count: usize,
        ) -> Poll<std::io::Result<usize>> {
            if self.0 {
                Poll::Ready(Ok(0))
            } else {
                self.0 = true;
                let read = count.min(2);
                output[index..index + read].copy_from_slice(&[7, 8][..read]);
                Poll::Ready(Ok(read))
            }
        }
    }

    let mut input = OneShotInput(false);
    let mut output = [0_u8; 3];
    let mut future = input.read_fully_async(&mut output);
    let mut cx = context();

    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("read_fully should complete")
        .expect("read_fully should succeed");
    assert_eq!(2, read);
}

#[test]
fn test_read_async_default_method_performs_one_read() {
    struct Input;

    impl AsyncInput for Input {
        type Item = u8;

        unsafe fn poll_read_unchecked(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            output: &mut [u8],
            index: usize,
            count: usize,
        ) -> Poll<std::io::Result<usize>> {
            let read = count.min(1);
            output[index..index + read].fill(9);
            Poll::Ready(Ok(read))
        }
    }

    let mut input = Input;
    let mut output = [0_u8; 2];
    let mut future = input.read_async(&mut output);
    let mut cx = context();

    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("read should complete")
        .expect("read should succeed");
    assert_eq!(1, read);
}

#[test]
fn test_read_fully_async_returns_non_interrupted_error() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![ReadStep::Error(
        ErrorKind::PermissionDenied,
    )]));
    let mut output = [0_u8; 2];
    let mut future = ReadFullyFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    let error = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("error should be ready")
        .expect_err("read_fully should return the input error");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}

#[test]
fn test_read_fully_async_completes_when_destination_is_full() {
    let mut input =
        Box::pin(ScriptedAsyncInput::new(vec![ReadStep::Data(vec![1, 2])]));
    let mut output = [0_u8; 2];
    let mut future = ReadFullyFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("full destination should complete")
        .expect("read_fully should succeed");

    assert_eq!(2, read);
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
