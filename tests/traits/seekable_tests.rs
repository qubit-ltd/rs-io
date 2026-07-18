// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::Seekable;
use std::io::Cursor;

#[test]
fn test_cursor_implements_seekable() {
    fn assert_seekable<T: Seekable>() {}
    assert_seekable::<Cursor<Vec<u8>>>();
}
