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

use super::support_tests::SafeOverreportedInput;
use super::support_tests::TestInput;

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
