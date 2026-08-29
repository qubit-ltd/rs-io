// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::FlushFuture;

use super::support_tests::ForbiddenErrorOutput;
use super::support_tests::TestOutput;

#[test]
fn test_flush_future_type_is_public() {
    assert!(std::any::type_name::<FlushFuture<'static, TestOutput>>().contains("FlushFuture"));
}

#[test]
fn test_flush_future_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut output = ForbiddenErrorOutput::new(kind);
        let mut future = FlushFuture::new(Pin::new(&mut output));
        let mut cx = Context::from_waker(Waker::noop());

        let error = match Future::poll(Pin::new(&mut future), &mut cx) {
            Poll::Ready(Err(error)) => error,
            result => {
                panic!("forbidden flush error should be ready: {result:?}")
            }
        };

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }
}
