// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, ErrorKind};

use qubit_io::Input;

struct OverreportingInput;

impl Input for OverreportingInput {
    type Item = u8;

    unsafe fn read_unchecked(
        &mut self,
        _output: &mut [u8],
        _index: usize,
        count: usize,
    ) -> std::io::Result<usize> {
        Ok(count + 1)
    }
}

#[test]
fn test_input_read_uses_default_validation() {
    let mut input = OverreportingInput;
    let mut output = [0_u8; 3];

    let error = input
        .read(&mut output)
        .expect_err("default read should validate reported counts");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_blanket_impl_exposes_input_read_and_read_unchecked() {
    let mut cursor = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 4];

    let read = Input::read(&mut cursor, &mut output).expect("read should succeed");
    assert_eq!(2, read);
    assert_eq!(b"ab\x00\x00", &output);

    let mut cursor = Cursor::new(b"cd".to_vec());
    let mut output = [b'.'; 4];
    // SAFETY: `output[1..3]` is a valid destination range.
    let read = unsafe {
        Input::read_unchecked(&mut cursor, &mut output, 1, 2)
            .expect("read_unchecked should succeed")
    };
    assert_eq!(2, read);
    assert_eq!(b".cd.", &output);
}
