// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::Pin;
use std::task::{
    Context,
    Waker,
};

use super::support_tests::TestOutput;
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
