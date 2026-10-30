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

use qubit_io::AsyncCountingOutput;
use qubit_io::AsyncOutput;

use super::support_tests::SafeOverreportOutput;
use super::support_tests::SafeProgressThenOverreportOutput;
use super::support_tests::TestStream;

#[test]
fn test_async_counting_output_starts_at_zero() {
    let output = AsyncCountingOutput::new(TestStream);
    assert_eq!(0, output.items_written());
}

#[test]
fn test_async_counting_output_rejects_safe_overreported_count() {
    let mut wrapper = AsyncCountingOutput::new(SafeOverreportOutput);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let result = AsyncOutput::poll_write(Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(0, wrapper.items_written());
}

#[test]
fn test_async_counting_output_preserves_short_progress_before_overreport() {
    let mut output = AsyncCountingOutput::new(SafeProgressThenOverreportOutput::default());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);

    assert!(matches!(
        AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, b"ab"),
        Poll::Ready(Ok(1))
    ));
    assert!(matches!(
        AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, b"ab"),
        Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData
    ));
    assert_eq!(1, output.items_written());
}
