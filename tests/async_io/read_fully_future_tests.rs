// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::ErrorKind;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::ReadFullyFuture;

use super::support_tests::PollResult;
use super::support_tests::ScriptedInput;
use super::support_tests::TestInput;

#[test]
fn test_read_fully_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadFullyFuture<'static, TestInput>>()
            .contains("ReadFullyFuture")
    );
}

#[test]
fn test_read_fully_future_constructs_and_reports_eof_progress() {
    let mut input = TestInput;
    let mut output = [0_u8; 1];
    let mut future = ReadFullyFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_read());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(0)) => {}
        Poll::Ready(Ok(read)) => panic!("expected EOF progress, got {read}"),
        Poll::Ready(Err(error)) => panic!("read should not fail: {error}"),
        Poll::Pending => panic!("test input should not pend"),
    }
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = Future::poll(Pin::new(&mut future), &mut cx);
        }))
        .is_err()
    );
}

#[test]
fn test_read_fully_future_supports_pending_reads_before_progress() {
    let mut input = ScriptedInput::new([
        PollResult::Pending,
        PollResult::Read(2),
        PollResult::Read(0),
    ]);
    let mut output = [0_u8; 4];
    let mut future = ReadFullyFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Pending
    ));
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(2)) => {}
        Poll::Ready(Ok(_)) => {
            panic!("read should return 2 bytes before EOF in this script")
        }
        Poll::Ready(Err(error)) => {
            panic!("read should succeed before EOF, got error: {error}")
        }
        Poll::Pending => {
            panic!("scripted input should make progress and return EOF")
        }
    }
}

#[test]
fn test_read_fully_future_constructs_and_propagates_io_error() {
    let mut input = ScriptedInput::new([
        PollResult::Read(1),
        PollResult::Error(std::io::Error::new(ErrorKind::TimedOut, "timeout")),
    ]);
    let mut output = [0_u8; 2];
    let mut future = ReadFullyFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::TimedOut, error.kind());
        }
        Poll::Ready(Ok(read)) => {
            panic!("read should not suppress I/O errors, got progress {read}")
        }
        Poll::Pending => panic!("scripted input should be immediately ready"),
    }
}

#[test]
fn test_read_fully_future_pauses_when_ready_budget_is_exhausted() {
    let mut input = ScriptedInput::new(
        std::iter::repeat_with(|| PollResult::Read(1)).take(65),
    );
    let mut output = [0_u8; 65];
    let mut future = ReadFullyFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Pending
    ));
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(65)) => {}
        Poll::Ready(Ok(_)) | Poll::Pending => {
            panic!(
                "read-fully future should complete after second poll, got not fully consumed"
            )
        }
        Poll::Ready(Err(error)) => {
            panic!(
                "read-fully future should complete after second poll, got error {error}"
            )
        }
    }
}
