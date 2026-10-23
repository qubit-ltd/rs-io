// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::type_name;
use std::future::Future;
use std::io::ErrorKind;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::WriteFuture;

use super::support_tests::SafeOverreportedOutput;
use super::support_tests::TestOutput;

#[test]
fn test_write_future_type_is_public() {
    assert!(type_name::<WriteFuture<'static, TestOutput>>().contains("WriteFuture"));
}

#[test]
fn test_write_future_rejects_safe_overreported_count() {
    let mut output = SafeOverreportedOutput::new(0);
    let input = [1_u8, 2];
    let mut future = WriteFuture::new(Pin::new(&mut output), &input);
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
