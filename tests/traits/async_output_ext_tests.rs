// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestOutput;
use qubit_io::AsyncOutputExt;

#[test]
fn test_async_output_ext_has_blanket_implementation() {
    fn assert_ext<T: AsyncOutputExt>() {}
    assert_ext::<TestOutput>();
}
