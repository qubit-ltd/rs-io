// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::ReadExactFuture;

#[test]
fn test_read_exact_future_type_is_public() {
    assert!(
        std::any::type_name::<ReadExactFuture<'static, TestInput>>().contains("ReadExactFuture")
    );
}
