// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::{
    BufferedOutput,
    EnsuredBufferedOutput,
};
use std::io::Cursor;

#[test]
fn test_ensured_buffered_output_reports_buffered_variant() {
    let output = BufferedOutput::new(Cursor::new(Vec::<u8>::new()));
    let ensured = BufferedOutput::ensure(output);
    assert!(matches!(ensured, EnsuredBufferedOutput::AlreadyBuffered(_)));
}
