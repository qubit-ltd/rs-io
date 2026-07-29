// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    future::Future,
    io::ErrorKind,
    panic::{
        AssertUnwindSafe,
        catch_unwind,
    },
    pin::Pin,
    task::{
        Context,
        Poll,
        Waker,
    },
};

use super::support_tests::{
    PollResult,
    ScriptedOutput,
    TestOutput,
};
use qubit_io::WriteFullyFuture;

#[test]
fn test_write_fully_future_type_is_public() {
    assert!(
        std::any::type_name::<WriteFullyFuture<'static, TestOutput>>()
            .contains("WriteFullyFuture")
    );
}

#[test]
fn test_write_fully_future_completes_immediately_for_empty_input() {
    let mut output = TestOutput;
    let input: [u8; 0] = [];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Pending => {
            panic!("empty input should never return pending")
        }
        Poll::Ready(Err(error)) => {
            panic!("empty write_fully should not error, got: {error}")
        }
    }
}

#[test]
fn test_write_fully_future_constructs_and_reports_progress() {
    let mut output = TestOutput;
    let input = [1_u8];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_written());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Ready(Err(error)) => panic!("write should not fail: {error}"),
        Poll::Pending => panic!("test output should not pend"),
    }
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = Future::poll(Pin::new(&mut future), &mut cx);
        }))
        .is_err()
    );
}

#[test]
fn test_write_fully_future_supports_pending_writes_before_progress() {
    let mut output = ScriptedOutput::new([
        PollResult::Pending,
        PollResult::Write(1),
        PollResult::Write(1),
    ]);
    let input = [0_u8; 2];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Pending
    ));
    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn test_write_fully_future_constructs_and_propagates_io_error() {
    let mut output = ScriptedOutput::new([
        PollResult::Write(1),
        PollResult::Error(std::io::Error::new(
            ErrorKind::BrokenPipe,
            "pipe broken",
        )),
    ]);
    let input = [0_u8; 3];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::BrokenPipe, error.kind());
        }
        Poll::Ready(Ok(())) => panic!("write should not suppress I/O errors"),
        Poll::Pending => panic!("scripted output should write immediately"),
    }
}

#[test]
fn test_write_fully_future_pauses_when_ready_budget_is_exhausted() {
    let mut output = ScriptedOutput::new(
        std::iter::repeat_with(|| PollResult::Write(1)).take(65),
    );
    let input = [0_u8; 65];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Pending
    ));
    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn test_write_fully_future_reports_write_zero_after_progress() {
    let mut output =
        ScriptedOutput::new([PollResult::Write(1), PollResult::Write(0)]);
    let input = [1_u8; 2];
    let mut future = WriteFullyFuture::new(Pin::new(&mut output), &input);
    let mut cx = Context::from_waker(Waker::noop());

    let error = match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => error,
        Poll::Ready(Ok(_)) => {
            panic!("partial write should not succeed after zero-acceptance")
        }
        Poll::Pending => panic!("expected WriteZero to return ready"),
    };
    assert_eq!(ErrorKind::WriteZero, error.kind());
}
