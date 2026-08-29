// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use qubit_io::BufferedInput;
use qubit_io::BufferedOutput;

/// Bounds allocations even when the target is invoked without CI flags.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_buffered_input(data);
    fuzz_buffered_output(data);
});

/// Exercises synchronous buffered reads while checking the logical stream
/// position and the unread-window invariant after every operation.
fn fuzz_buffered_input(data: &[u8]) {
    let capacity = usize::from(data.first().copied().unwrap_or_default() % 32) + 1;
    let source = data.get(1..).unwrap_or_default();
    let mut input = BufferedInput::with_capacity(Cursor::new(source.to_vec()), capacity);
    let mut position = 0;

    for operation in source.chunks(3) {
        let opcode = operation.first().copied().unwrap_or_default() % 3;
        let argument = usize::from(operation.get(1).copied().unwrap_or_default());
        match opcode {
            0 if input.unread_len() < capacity => {
                input.fill_more().expect("cursor reads should not fail");
            }
            1 => {
                let count = argument % (capacity + 1);
                let filled = input.fill_until(count).expect("cursor reads should not fail");
                assert_eq!(source.len().saturating_sub(position) >= count, filled);
            }
            _ => {
                let count = argument % 33;
                let mut output = vec![0_u8; count];
                let read = input.read_fully(&mut output).expect("cursor reads should not fail");
                let expected = (source.len() - position).min(count);
                assert_eq!(expected, read);
                assert_eq!(&source[position..position + read], &output[..read]);
                position += read;
            }
        }
        assert_eq!(input.unread(), &source[position..position + input.unread_len()]);
    }
}

/// Exercises synchronous buffered writes and validates the wrapped output after
/// each explicit flush.
fn fuzz_buffered_output(data: &[u8]) {
    let capacity = usize::from(data.first().copied().unwrap_or_default() % 32) + 1;
    let mut output = BufferedOutput::with_capacity(Cursor::new(Vec::new()), capacity);
    let mut expected = Vec::new();

    for operation in data.get(1..).unwrap_or_default().chunks(3) {
        let opcode = operation.first().copied().unwrap_or_default() % 3;
        let count = usize::from(operation.get(1).copied().unwrap_or_default() % 33);
        let value = operation.get(2).copied().unwrap_or_default();
        match opcode {
            0 => {
                let values = vec![value; count];
                output.write_fully(&values).expect("cursor writes should not fail");
                expected.extend_from_slice(&values);
            }
            1 => output
                .ensure_spare_capacity(count % (capacity + 1))
                .expect("cursor writes should not fail"),
            _ => {
                output.flush().expect("cursor flushes should not fail");
                assert_eq!(&expected, output.inner().get_ref());
            }
        }
    }

    output.flush().expect("cursor flushes should not fail");
    assert_eq!(&expected, output.inner().get_ref());
}
