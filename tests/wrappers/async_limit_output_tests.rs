// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestStream;
use qubit_io::AsyncLimitOutput;

#[test]
fn test_async_limit_output_exposes_initial_limit() {
    let output = AsyncLimitOutput::new(TestStream, 7);
    assert_eq!(7, output.remaining());
}
