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
use std::task::{
    Context,
    Poll,
    Waker,
};

use qubit_io::{
    AsyncInput,
    AsyncInputExt,
    ReadFullyFuture,
};

enum ReadStep {
    Data(Vec<u8>),
    Error(ErrorKind),
    Interrupted,
    Pending,
    Eof,
}

struct ScriptedAsyncInput {
    steps: VecDeque<ReadStep>,
    _pinned: PhantomPinned,
}

impl ScriptedAsyncInput {
    fn new(steps: Vec<ReadStep>) -> Self {
        Self {
            steps: VecDeque::from(steps),
            _pinned: PhantomPinned,
        }
    }
}

impl AsyncInput for ScriptedAsyncInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: This implementation does not move the pinned value.
        let this = unsafe { self.get_unchecked_mut() };
        match this.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(data) => {
                let read = count.min(data.len());
                output[index..index + read].copy_from_slice(&data[..read]);
                Poll::Ready(Ok(read))
            }
            ReadStep::Error(kind) => {
                Poll::Ready(Err(Error::new(kind, "read failed")))
            }
            ReadStep::Interrupted => Poll::Ready(Err(Error::new(
                ErrorKind::Interrupted,
                "interrupted",
            ))),
            ReadStep::Pending => Poll::Pending,
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
fn test_read_fully_async_handles_pending_partial_interrupted_and_eof() {
    let mut input = Box::pin(ScriptedAsyncInput::new(vec![
        ReadStep::Data(vec![1, 2]),
        ReadStep::Pending,
        ReadStep::Interrupted,
        ReadStep::Data(vec![3]),
        ReadStep::Eof,
    ]));
    let mut output = [0_u8; 4];
    let mut future = ReadFullyFuture::new(input.as_mut(), &mut output);
    let mut cx = context();

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    let read = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("second poll should complete")
        .expect("read_fully should succeed");
    assert_eq!(3, read);
    drop(future);
    assert_eq!([1, 2, 3, 0], output);
}

#[test]
fn test_read_fully_async_extension_method_reads_unpin_input() {
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
fn test_read_async_extension_method_performs_one_read() {
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
