// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use qubit_io::AsyncChecksumOutput;

use super::support_tests::TestStream;

#[test]
fn test_async_checksum_output_starts_with_hasher_checksum() {
    let hasher = DefaultHasher::new();
    let expected = hasher.finish();
    let output = AsyncChecksumOutput::new(TestStream, hasher);
    assert_eq!(expected, output.checksum());
}

#[test]
fn test_async_checksum_output_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::AsyncChecksumOutput::new(
        super::support_tests::SafeOverreportOutput,
        std::collections::hash_map::DefaultHasher::new(),
    );
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let before = wrapper.checksum();
    let result = qubit_io::AsyncOutput::poll_write(std::pin::Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(
        matches!(result, std::task::Poll::Ready(Err(ref error)) if error.kind() == std::io::ErrorKind::InvalidData)
    );
    assert_eq!(before, wrapper.checksum());
}
