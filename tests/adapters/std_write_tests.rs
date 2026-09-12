// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use std::io::Cursor;
use std::io::Write;

use qubit_io::StdWrite;

#[test]
fn test_std_write_delegates_to_output() {
    let mut writer = StdWrite::new(Cursor::new(Vec::<u8>::new()));
    writer.write_all(b"abcd").expect("writer should accept input");
    writer.flush().expect("writer should flush");
    assert_eq!(writer.get_ref().get_ref(), b"abcd");
    assert_eq!(writer.into_inner().into_inner(), b"abcd");
}
