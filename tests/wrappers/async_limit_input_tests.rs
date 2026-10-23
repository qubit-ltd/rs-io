// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::AsyncLimitInput;

use super::support_tests::TestStream;

#[test]
fn test_async_limit_input_exposes_initial_limit() {
    let input = AsyncLimitInput::new(TestStream, 7);
    assert_eq!(7, input.remaining());
}

#[test]
fn test_async_limit_input_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::AsyncLimitInput::new(super::support_tests::SafeOverreportInput, 1);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut bytes = [0_u8; 2];
    let result = qubit_io::AsyncInput::poll_read(std::pin::Pin::new(&mut wrapper), &mut cx, &mut bytes);
    assert!(
        matches!(result, std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(1, wrapper.remaining());
}
