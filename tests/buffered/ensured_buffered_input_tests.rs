// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::{
    BufferedInput,
    EnsuredBufferedInput,
};
use std::io::Cursor;

#[test]
fn test_ensured_buffered_input_reports_buffered_variant() {
    let input = BufferedInput::new(Cursor::new(Vec::<u8>::new()));
    let ensured = BufferedInput::ensure(input);
    assert!(matches!(ensured, EnsuredBufferedInput::AlreadyBuffered(_)));
}
