// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for the standard [`Seek`](std::io::Seek) bridge to [`Seekable`].

use std::io::Cursor;

use qubit_io::Seekable;

#[test]
fn test_cursor_implements_seekable() {
    fn assert_seekable<T: Seekable>() {}
    assert_seekable::<Cursor<Vec<u8>>>();
}
