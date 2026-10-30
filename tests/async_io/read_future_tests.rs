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

use qubit_io::ReadFuture;

use super::support_tests::PollResult;
use super::support_tests::SafeErrorInput;
use super::support_tests::SafeOverreportedInput;
use super::support_tests::SafeScriptedInput;
use super::support_tests::TestInput;
use super::support_tests::assert_normalized_error;

#[test]
fn test_read_future_type_is_public() {
    assert!(std::any::type_name::<ReadFuture<'static, TestInput>>().contains("ReadFuture"));
}

#[test]
fn test_read_future_rejects_safe_overreported_count() {
    let mut input = SafeOverreportedInput::new(0);
    let mut output = [0_u8; 2];
    let mut future = ReadFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => assert_eq!(ErrorKind::InvalidData, error.kind()),
        other => panic!("expected InvalidData, got {other:?}"),
    }
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = Future::poll(Pin::new(&mut future), &mut cx);
        }))
        .is_err()
    );
}

#[test]
fn test_read_future_normalizes_safe_override_would_block() {
    let mut input = SafeErrorInput(std::io::Error::new(ErrorKind::WouldBlock, "original would-block"));
    let mut output = [0_u8; 1];
    let mut future = ReadFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::InvalidData, error.kind());
            assert!(
                error
                    .get_ref()
                    .and_then(std::error::Error::source)
                    .is_some_and(|source| source.to_string().contains("original would-block"))
            );
        }
        other => panic!("expected normalized error, got {other:?}"),
    }
}

#[test]
fn test_read_future_normalizes_both_forbidden_errors_and_preserves_ordinary_error() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = SafeScriptedInput::new([PollResult::Error(std::io::Error::new(kind, "read marker"))]);
        let mut output = [0_u8; 1];
        let mut future = ReadFuture::new(Pin::new(&mut input), &mut output);
        let mut cx = Context::from_waker(Waker::noop());
        match Future::poll(Pin::new(&mut future), &mut cx) {
            Poll::Ready(Err(error)) => assert_normalized_error(&error, kind, "read marker"),
            other => panic!("expected normalized error, got {other:?}"),
        }
    }

    let mut input = SafeScriptedInput::new([PollResult::Error(std::io::Error::new(
        ErrorKind::PermissionDenied,
        "read payload",
    ))]);
    let mut output = [0_u8; 1];
    let mut future = ReadFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::PermissionDenied, error.kind());
            assert_eq!("read payload", error.to_string());
        }
        other => panic!("expected original error, got {other:?}"),
    }
}
