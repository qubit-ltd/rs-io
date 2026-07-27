// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use super::support_tests::{ForbiddenErrorOutput, TestOutput};
use qubit_io::CloseFuture;

#[test]
fn test_close_future_type_is_public_and_panics_after_completion() {
    let mut output = TestOutput;
    let mut future = CloseFuture::new(Pin::new(&mut output));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Future::poll(Pin::new(&mut future), &mut cx);
    }));
    assert!(result.is_err());
}

#[test]
fn test_close_future_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut output = ForbiddenErrorOutput::new(kind);
        let mut future = CloseFuture::new(Pin::new(&mut output));
        let mut cx = Context::from_waker(Waker::noop());

        let error = match Future::poll(Pin::new(&mut future), &mut cx) {
            Poll::Ready(Err(error)) => error,
            result => {
                panic!("forbidden close error should be ready: {result:?}")
            }
        };

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }
}
