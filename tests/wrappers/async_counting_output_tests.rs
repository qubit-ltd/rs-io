// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::AsyncCountingOutput;

use super::support_tests::TestStream;

#[test]
fn test_async_counting_output_starts_at_zero() {
    let output = AsyncCountingOutput::new(TestStream);
    assert_eq!(0, output.items_written());
}

#[test]
fn test_async_counting_output_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::AsyncCountingOutput::new(super::support_tests::SafeOverreportOutput);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let result = qubit_io::AsyncOutput::poll_write(std::pin::Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(
        matches!(result, std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(0, wrapper.items_written());
}

#[test]
fn test_async_counting_output_preserves_short_progress_before_overreport() {
    let mut output =
        qubit_io::AsyncCountingOutput::new(super::support_tests::SafeProgressThenOverreportOutput::default());
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);

    assert!(matches!(
        qubit_io::AsyncOutput::poll_write(std::pin::Pin::new(&mut output), &mut cx, b"ab"),
        std::task::Poll::Ready(Ok(1))
    ));
    assert!(
        matches!(qubit_io::AsyncOutput::poll_write(std::pin::Pin::new(&mut output), &mut cx, b"ab"), std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(1, output.items_written());
}
