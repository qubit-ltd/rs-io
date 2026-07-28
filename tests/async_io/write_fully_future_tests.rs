// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestOutput;
use qubit_io::WriteFullyFuture;
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[test]
fn test_write_fully_future_type_is_public() {
    assert!(
        std::any::type_name::<WriteFullyFuture<'static, TestOutput>>().contains("WriteFullyFuture")
    );
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
