// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, Error, ErrorKind, Write};

use qubit_io::Output;

struct OverreportingStdWriter;

struct FailingStdWriter;

impl Write for FailingStdWriter {
    /// Returns a deterministic standard writer failure.
    fn write(&mut self, _input: &[u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::BrokenPipe, "write failed"))
    }

    /// Completes flushing because the test writer buffers no data.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for OverreportingStdWriter {
    /// Deliberately violates the standard writer count contract.
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        Ok(input.len() + 1)
    }

    /// Completes flushing because the test writer buffers no data.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Verifies normal, indexed, and flush operations through the standard writer bridge.
#[test]
fn test_write_blanket_impl_exposes_output_methods() {
    let mut cursor = Cursor::new(Vec::new());

    // SAFETY: b"bc" is a valid source range inside b"abc".
    let written = unsafe {
        Output::write_unchecked(&mut cursor, b"abc", 1, 2).expect("write_unchecked should succeed")
    };
    assert_eq!(2, written);
    assert_eq!(b"bc", cursor.into_inner().as_slice());

    let mut cursor = Cursor::new(Vec::new());
    let written = Output::write(&mut cursor, b"xy")
        .expect("Output::write should delegate to write_unchecked");
    assert_eq!(2, written);
    assert_eq!(b"xy", cursor.into_inner().as_slice());

    let mut cursor = Cursor::new(Vec::new());
    Write::write_all(&mut cursor, b"z").expect("seed bytes for flush");
    Output::flush(&mut cursor).expect("flush should succeed");
}

/// Verifies that the standard writer bridge rejects impossible write counts.
#[test]
fn test_write_blanket_impl_rejects_overreported_count() {
    let mut writer = OverreportingStdWriter;

    let error = Output::write(&mut writer, b"abc")
        .expect_err("blanket output should validate the Write count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

/// Verifies that the standard writer bridge preserves writer errors.
#[test]
fn test_write_blanket_impl_propagates_std_write_error() {
    let mut writer = FailingStdWriter;

    let error = Output::write(&mut writer, b"abc")
        .expect_err("blanket output should propagate Write errors");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
}
