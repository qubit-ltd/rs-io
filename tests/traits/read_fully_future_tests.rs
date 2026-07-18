// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::ReadFullyFuture;

#[test]
fn test_read_fully_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadFullyFuture<'static, TestInput>>()
            .contains("ReadFullyFuture")
    );
}
