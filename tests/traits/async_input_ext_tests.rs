// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestInput;
use qubit_io::AsyncInputExt;

#[test]
fn test_async_input_ext_has_blanket_implementation() {
    fn assert_ext<T: AsyncInputExt>() {}
    assert_ext::<TestInput>();
}
