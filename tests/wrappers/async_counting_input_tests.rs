// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::AsyncCountingInput;

use super::support_tests::TestStream;

#[test]
fn test_async_counting_input_starts_at_zero() {
    let input = AsyncCountingInput::new(TestStream);
    assert_eq!(0, input.items_read());
}

#[test]
fn test_async_counting_input_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::AsyncCountingInput::new(super::support_tests::SafeOverreportInput);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut bytes = [0_u8; 2];
    let result = qubit_io::AsyncInput::poll_read(std::pin::Pin::new(&mut wrapper), &mut cx, &mut bytes);
    assert!(
        matches!(result, std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(0, wrapper.items_read());
}

#[test]
fn test_async_counting_input_preserves_short_progress_before_overreport() {
    let mut input = qubit_io::AsyncCountingInput::new(super::support_tests::SafeProgressThenOverreportInput::default());
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut output = [0_u8; 2];

    assert!(matches!(
        qubit_io::AsyncInput::poll_read(std::pin::Pin::new(&mut input), &mut cx, &mut output),
        std::task::Poll::Ready(Ok(1))
    ));
    assert!(
        matches!(qubit_io::AsyncInput::poll_read(std::pin::Pin::new(&mut input), &mut cx, &mut output), std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(1, input.items_read());
}
