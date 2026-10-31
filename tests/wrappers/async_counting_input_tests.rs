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

use qubit_io::AsyncCountingInput;
use qubit_io::AsyncInput;

use super::support_tests::SafeOverreportInput;
use super::support_tests::SafeProgressThenOverreportInput;
use super::support_tests::TestStream;

#[test]
fn test_async_counting_input_starts_at_zero() {
    let input = AsyncCountingInput::new(TestStream);
    assert_eq!(0, input.items_read());
}

#[test]
fn test_async_counting_input_rejects_safe_overreported_count() {
    let mut wrapper = AsyncCountingInput::new(SafeOverreportInput);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut bytes = [0_u8; 2];
    let result = AsyncInput::poll_read(Pin::new(&mut wrapper), &mut cx, &mut bytes);
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(0, wrapper.items_read());
}

#[test]
fn test_async_counting_input_preserves_short_progress_before_overreport() {
    let mut input = AsyncCountingInput::new(SafeProgressThenOverreportInput::default());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut output = [0_u8; 2];

    assert!(matches!(
        AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output),
        Poll::Ready(Ok(1))
    ));
    assert!(matches!(
        AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output),
        Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData
    ));
    assert_eq!(1, input.items_read());
}
