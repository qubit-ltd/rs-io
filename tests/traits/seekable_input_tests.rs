// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_io::SeekableInput;

#[test]
fn test_cursor_implements_seekable_input() {
    fn assert_seekable_input<T: SeekableInput>() {}
    assert_seekable_input::<Cursor<Vec<u8>>>();
}
