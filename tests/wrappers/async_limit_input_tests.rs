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
