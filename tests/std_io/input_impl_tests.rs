// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, Error, ErrorKind, Read};

use qubit_io::Input;

struct OverreportingStdReader;

struct FailingStdReader;

impl Read for FailingStdReader {
    /// Returns a deterministic standard reader failure.
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::PermissionDenied, "read failed"))
    }
}

impl Read for OverreportingStdReader {
    /// Deliberately violates the standard reader count contract.
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        Ok(output.len() + 1)
    }
}

/// Verifies normal and indexed reads through the standard reader bridge.
#[test]
fn test_read_blanket_impl_exposes_input_read_and_read_unchecked() {
    let mut cursor = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 4];

    let read = Input::read(&mut cursor, &mut output).expect("read should succeed");
    assert_eq!(2, read);
    assert_eq!(b"ab\x00\x00", &output);

    let mut cursor = Cursor::new(b"cd".to_vec());
    let mut output = [b'.'; 4];
    // SAFETY: output[1..3] is a valid destination range.
    let read = unsafe {
        Input::read_unchecked(&mut cursor, &mut output, 1, 2)
            .expect("read_unchecked should succeed")
    };
    assert_eq!(2, read);
    assert_eq!(b".cd.", &output);
}

/// Verifies that the standard reader bridge rejects impossible read counts.
#[test]
fn test_read_blanket_impl_rejects_overreported_count() {
    let mut reader = OverreportingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output)
        .expect_err("blanket input should validate the Read count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

/// Verifies that the standard reader bridge preserves reader errors.
#[test]
fn test_read_blanket_impl_propagates_std_read_error() {
    let mut reader = FailingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output)
        .expect_err("blanket input should propagate Read errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}
