// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::AsyncBufferedInput;

#[test]
fn test_async_buffered_input_uses_requested_capacity() {
    let input = AsyncBufferedInput::with_capacity(TestInput, 3);
    assert_eq!(3, input.capacity());
}
