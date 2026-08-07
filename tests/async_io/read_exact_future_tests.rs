// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
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

use qubit_io::ReadExactFuture;

use super::support_tests::PollResult;
use super::support_tests::ScriptedInput;
use super::support_tests::TestInput;

#[test]
fn test_read_exact_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadExactFuture<'static, TestInput>>()
            .contains("ReadExactFuture")
    );
}

#[test]
fn test_read_exact_future_completes_immediately_for_empty_destination() {
    let mut input = TestInput;
    let mut output: [u8; 0] = [];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Pending => {
            panic!("empty destination should never return pending")
        }
        Poll::Ready(Err(error)) => {
            panic!("empty read_exact should not error, got: {error}")
        }
    }
}

#[test]
fn test_read_exact_future_constructs_and_reports_eof() {
    let mut input = TestInput;
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_read());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(std::io::ErrorKind::UnexpectedEof, error.kind());
        }
        Poll::Ready(Ok(())) => panic!("exact read should report EOF"),
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
fn test_read_exact_future_supports_pending_reads_before_progress() {
    let mut input = ScriptedInput::new([
        PollResult::Pending,
        PollResult::Read(1),
        PollResult::Read(1),
    ]);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
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
fn test_read_exact_future_constructs_and_propagates_io_error() {
    let mut input = ScriptedInput::new([PollResult::Error(
        std::io::Error::new(ErrorKind::PermissionDenied, "denied"),
    )]);
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::PermissionDenied, error.kind());
        }
        Poll::Ready(Ok(())) => {
            panic!("exact read should not swallow I/O errors")
        }
        Poll::Pending => panic!("scripted input should be immediately ready"),
    }
}

#[test]
fn test_read_exact_future_pauses_when_ready_budget_is_exhausted() {
    let mut input = ScriptedInput::new(
        std::iter::repeat_with(|| PollResult::Read(1)).take(65),
    );
    let mut output = [0_u8; 65];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
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
fn test_read_exact_future_reports_eof_after_partial_progress() {
    let mut input =
        ScriptedInput::new([PollResult::Read(1), PollResult::Read(0)]);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_read());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::UnexpectedEof, error.kind());
            assert_eq!(1, future.items_read());
        }
        Poll::Pending => panic!("partial read should not remain pending"),
        Poll::Ready(Ok(_)) => {
            panic!("partial read should fail with UnexpectedEof")
        }
    }
}
