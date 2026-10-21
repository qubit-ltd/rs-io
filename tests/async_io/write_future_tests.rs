// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::type_name;

use qubit_io::WriteFuture;

use super::support_tests::TestOutput;

#[test]
fn test_write_future_type_is_public() {
    assert!(type_name::<WriteFuture<'static, TestOutput>>().contains("WriteFuture"));
}
