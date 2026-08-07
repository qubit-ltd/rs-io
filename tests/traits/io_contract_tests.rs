// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncInput;

struct ForbiddenErrorInput {
    kind: ErrorKind,
}

impl AsyncInput for ForbiddenErrorInput {
    type Item = u8;

    /// Returns the configured forbidden asynchronous error.
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden async error")))
    }
}

/// Verifies that forbidden asynchronous error kinds are normalized.
#[test]
fn test_async_contract_normalizes_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = ForbiddenErrorInput { kind };
        let mut output = [0_u8; 1];
        let mut cx = Context::from_waker(Waker::noop());
        let result =
            AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output);

        match result {
            Poll::Ready(Err(error)) => {
                assert_eq!(ErrorKind::InvalidData, error.kind());
            }
            Poll::Ready(Ok(_)) => panic!("forbidden async error should fail"),
            Poll::Pending => panic!("forbidden async error should be ready"),
        }
    }
}
