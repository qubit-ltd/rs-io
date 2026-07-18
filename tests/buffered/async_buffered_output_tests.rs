// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestOutput;
use qubit_io::AsyncBufferedOutput;

#[test]
fn test_async_buffered_output_uses_requested_capacity() {
    let output = AsyncBufferedOutput::with_capacity(TestOutput, 3);
    assert_eq!(3, output.capacity());
}
