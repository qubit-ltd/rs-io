// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncLimitOutput;
use qubit_io::AsyncOutput;

use super::support_tests::SafeOverreportOutput;
use super::support_tests::TestStream;

#[test]
fn test_async_limit_output_exposes_initial_limit() {
    let output = AsyncLimitOutput::new(TestStream, 7);
    assert_eq!(7, output.remaining());
}

#[test]
fn test_async_limit_output_rejects_safe_overreported_count() {
    let mut wrapper = AsyncLimitOutput::new(SafeOverreportOutput, 1);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let result = AsyncOutput::poll_write(Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(1, wrapper.remaining());
}
