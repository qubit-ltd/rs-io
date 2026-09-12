// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::io::Cursor;
use std::io::Read;

use qubit_io::StdRead;

#[test]
fn test_std_read_delegates_to_input() {
    let mut reader = StdRead::new(Cursor::new(b"abcd"));
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).expect("reader should fill output");
    assert_eq!(&bytes, b"abcd");
    assert_eq!(reader.into_inner().position(), 4);
}
