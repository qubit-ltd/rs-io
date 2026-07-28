// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::ReadExactFuture;
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[test]
fn test_read_exact_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadExactFuture<'static, TestInput>>().contains("ReadExactFuture")
    );
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
