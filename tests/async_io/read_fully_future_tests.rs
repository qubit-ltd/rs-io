// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::ReadFullyFuture;
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[test]
fn test_read_fully_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadFullyFuture<'static, TestInput>>().contains("ReadFullyFuture")
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
