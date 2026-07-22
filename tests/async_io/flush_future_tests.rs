// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestOutput;
use qubit_io::FlushFuture;

#[test]
fn test_flush_future_type_is_public() {
    assert!(
        std::any::type_name::<FlushFuture<'static, TestOutput>>()
            .contains("FlushFuture")
    );
}
