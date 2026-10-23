// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::AsyncLimitOutput;

use super::support_tests::TestStream;

#[test]
fn test_async_limit_output_exposes_initial_limit() {
    let output = AsyncLimitOutput::new(TestStream, 7);
    assert_eq!(7, output.remaining());
}

#[test]
fn test_async_limit_output_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::AsyncLimitOutput::new(super::support_tests::SafeOverreportOutput, 1);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let result = qubit_io::AsyncOutput::poll_write(std::pin::Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(
        matches!(result, std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(1, wrapper.remaining());
}
