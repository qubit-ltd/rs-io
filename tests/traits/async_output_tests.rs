// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Context, Poll, Wake, Waker};

use qubit_io::{AsyncOutput, WriteFullyFuture};

enum WriteStep {
    Accept(usize),
    Error(ErrorKind),
    Pending,
    Zero,
}

struct ScriptedAsyncOutput {
    values: Vec<u8>,
    write_steps: VecDeque<WriteStep>,
    poll_count: usize,
    registered_waker: Option<Waker>,
    flush_pending: bool,
    _pinned: PhantomPinned,
}

impl ScriptedAsyncOutput {
    fn new(write_steps: Vec<WriteStep>) -> Self {
        Self {
            values: Vec::new(),
            write_steps: VecDeque::from(write_steps),
            poll_count: 0,
            registered_waker: None,
            flush_pending: false,
            _pinned: PhantomPinned,
        }
    }
}

impl AsyncOutput for ScriptedAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: This implementation does not move the pinned value.
        let this = unsafe { self.get_unchecked_mut() };
        this.poll_count += 1;
        match this
            .write_steps
            .pop_front()
            .unwrap_or(WriteStep::Accept(count))
        {
            WriteStep::Accept(limit) => {
                let written = count.min(limit);
                this.values
                    .extend_from_slice(&input[index..index + written]);
                Poll::Ready(Ok(written))
            }
            WriteStep::Error(kind) => Poll::Ready(Err(Error::new(kind, "write failed"))),
            WriteStep::Pending => {
                this.registered_waker = Some(cx.waker().clone());
                Poll::Pending
            }
            WriteStep::Zero => Poll::Ready(Ok(0)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: This implementation does not move the pinned value.
        let this = unsafe { self.get_unchecked_mut() };
        if this.flush_pending {
            this.flush_pending = false;
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

struct OverreportingAsyncOutput;

impl AsyncOutput for OverreportingAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(count + 1))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct OneItemAsyncOutput {
    values: Vec<u8>,
}

impl AsyncOutput for OneItemAsyncOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        self.values.push(input[index]);
        Poll::Ready(Ok(1))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_async_output_is_object_safe_and_defaults_to_unbuffered() {
    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![]));
    let output: Pin<&mut dyn AsyncOutput<Item = u8>> = output.as_mut();

    assert!(!output.is_buffered());
}

#[test]
fn test_async_output_poll_write_rejects_overreported_count() {
    let mut output = OverreportingAsyncOutput;
    let mut cx = context();

    let result = AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, &[1, 2, 3]);

    let error = result
        .expect_ready("overreported write should be ready")
        .expect_err("overreported write should fail");
    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_async_output_zero_length_write_does_not_poll_inner() {
    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![]));
    let mut cx = context();

    let result = AsyncOutput::poll_write(output.as_mut(), &mut cx, &[]);

    assert!(matches!(result, Poll::Ready(Ok(0))));
    assert_eq!(0, output.poll_count);
}

#[test]
fn test_async_output_poll_write_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut output = Box::pin(ScriptedAsyncOutput::new(vec![WriteStep::Error(kind)]));
        let mut cx = context();

        let error = AsyncOutput::poll_write(output.as_mut(), &mut cx, &[1])
            .expect_ready("forbidden write error should be ready")
            .expect_err("forbidden write error should fail");

        assert_eq!(ErrorKind::InvalidData, error.kind());
        assert!(output.values.is_empty());
    }
}

#[test]
fn test_async_output_pending_registers_waker_without_progress() {
    struct TestWake(AtomicBool);

    impl Wake for TestWake {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![WriteStep::Pending]));
    let wake_state = Arc::new(TestWake(AtomicBool::new(false)));
    let waker = Waker::from(Arc::clone(&wake_state));
    let mut cx = Context::from_waker(&waker);

    let result = AsyncOutput::poll_write(output.as_mut(), &mut cx, &[1]);

    assert!(result.is_pending());
    assert!(output.values.is_empty());
    let registered_waker = output
        .registered_waker
        .as_ref()
        .expect("pending write should register the current waker");
    registered_waker.wake_by_ref();
    assert!(wake_state.0.load(Ordering::Relaxed));
}

#[test]
fn test_write_fully_async_handles_pending_and_partial_writes() {
    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![
        WriteStep::Accept(2),
        WriteStep::Pending,
        WriteStep::Accept(2),
    ]));
    let mut future = WriteFullyFuture::new(output.as_mut(), &[1, 2, 3, 4]);
    let mut cx = context();

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    assert_eq!(2, future.items_written());
    Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("second poll should complete")
        .expect("write_fully should succeed");
    drop(future);
    assert_eq!(&[1, 2, 3, 4], output.values.as_slice());
}

#[test]
fn test_write_fully_async_reports_write_zero() {
    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![WriteStep::Zero]));
    let mut future = WriteFullyFuture::new(output.as_mut(), &[1, 2, 3]);
    let mut cx = context();

    let error = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("zero write should be ready")
        .expect_err("zero write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_write_fully_async_returns_non_interrupted_error() {
    let mut output = Box::pin(ScriptedAsyncOutput::new(vec![WriteStep::Error(
        ErrorKind::BrokenPipe,
    )]));
    let mut future = WriteFullyFuture::new(output.as_mut(), &[1, 2, 3]);
    let mut cx = context();

    let error = Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("error should be ready")
        .expect_err("write_fully should return the output error");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
}

#[test]
fn test_write_fully_async_default_method_writes_all_items() {
    struct Output(Vec<u8>);

    impl AsyncOutput for Output {
        type Item = u8;

        unsafe fn poll_write_unchecked(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            input: &[u8],
            index: usize,
            count: usize,
        ) -> Poll<std::io::Result<usize>> {
            self.0.extend_from_slice(&input[index..index + count]);
            Poll::Ready(Ok(count))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    let mut output = Output(Vec::new());
    let input = [1, 2, 3];
    let mut future = output.write_fully_async(&input);
    let mut cx = context();

    Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("write_fully should complete")
        .expect("write_fully should succeed");
    drop(future);
    assert_eq!(&[1, 2, 3], output.0.as_slice());
}

#[test]
fn test_write_fully_async_yields_after_ready_progress_budget() {
    const ITEM_COUNT: usize = 256;

    struct TestWake(AtomicBool);

    impl Wake for TestWake {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    let mut output = OneItemAsyncOutput { values: Vec::new() };
    let input = [7_u8; ITEM_COUNT];
    let wake_state = Arc::new(TestWake(AtomicBool::new(false)));
    let waker = Waker::from(Arc::clone(&wake_state));
    let mut cx = Context::from_waker(&waker);
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    assert!(wake_state.0.load(Ordering::Relaxed));
    assert!(future.items_written() > 0);
    assert!(future.items_written() < ITEM_COUNT);

    let mut completed = false;
    for _ in 0..8 {
        if let Poll::Ready(result) = Future::poll(Pin::new(&mut future), &mut cx) {
            result.expect("cooperative write should succeed");
            completed = true;
            break;
        }
    }
    assert!(completed);
    drop(future);
    assert_eq!(input.as_slice(), output.values.as_slice());
}

#[test]
fn test_async_output_default_methods_support_write_and_flush() {
    struct Output {
        values: Vec<u8>,
        pending_flush: bool,
    }

    impl AsyncOutput for Output {
        type Item = u8;

        unsafe fn poll_write_unchecked(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            input: &[u8],
            index: usize,
            count: usize,
        ) -> Poll<std::io::Result<usize>> {
            self.values.extend_from_slice(&input[index..index + count]);
            Poll::Ready(Ok(count))
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            if self.pending_flush {
                self.pending_flush = false;
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }

    let mut output = Output {
        values: Vec::new(),
        pending_flush: true,
    };
    let mut cx = context();
    {
        let input = [4, 5];
        let mut future = output.write_async(&input);
        let written = Future::poll(Pin::new(&mut future), &mut cx)
            .expect_ready("write should complete")
            .expect("write should succeed");
        assert_eq!(2, written);
    }
    {
        let mut future = output.flush_async();
        assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
        Future::poll(Pin::new(&mut future), &mut cx)
            .expect_ready("second flush poll should complete")
            .expect("flush should succeed");
    }
    assert_eq!(&[4, 5], output.values.as_slice());
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
