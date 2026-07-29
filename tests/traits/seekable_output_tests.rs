// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_io::SeekableOutput;

#[test]
fn test_cursor_implements_seekable_output() {
    fn assert_seekable_output<T: SeekableOutput>() {}
    assert_seekable_output::<Cursor<Vec<u8>>>();
}
