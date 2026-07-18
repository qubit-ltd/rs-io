// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestStream;
use qubit_io::AsyncCountingInput;

#[test]
fn test_async_counting_input_starts_at_zero() {
    let input = AsyncCountingInput::new(TestStream);
    assert_eq!(0, input.items_read());
}
