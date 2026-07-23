// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_io::Buffer;

fuzz_target!(|data: &[u8]| {
    let capacity =
        usize::from(data.first().copied().unwrap_or_default() % 32) + 1;
    let mut buffer = Buffer::<u8>::with_capacity(capacity);
    let mut model = Vec::new();

    for operation in data.chunks(3) {
        let opcode = operation.first().copied().unwrap_or_default() % 7;
        let argument = operation.get(1).copied().unwrap_or_default();
        let value = operation.get(2).copied().unwrap_or_default();
        match opcode {
            0 => append_with_copy(&mut buffer, &mut model, value),
            1 => copy_out(&mut buffer, &mut model, argument),
            2 => consume(&mut buffer, &mut model, argument),
            3 => buffer.compact(),
            4 => {
                buffer.clear();
                model.clear();
            }
            5 => {
                let capacity = usize::from(argument % 128) + 1;
                buffer
                    .try_reserve_capacity(capacity)
                    .expect("small fuzz capacities should reserve");
            }
            _ => append_through_spare(&mut buffer, &mut model, value),
        }
        assert_invariants(&buffer, &model);
    }
});

/// Makes at least one spare slot available when practical.
fn ensure_spare(buffer: &mut Buffer<u8>) {
    if buffer.spare_capacity() == 0 {
        buffer.compact();
    }
    if buffer.spare_capacity() == 0 {
        buffer
            .try_reserve_capacity(buffer.capacity() + 1)
            .expect("one additional fuzz slot should reserve");
    }
}

/// Appends one modeled value through the unchecked copy API.
fn append_with_copy(buffer: &mut Buffer<u8>, model: &mut Vec<u8>, value: u8) {
    ensure_spare(buffer);
    // SAFETY: The one-element source and one spare destination slot are valid
    // and belong to separate allocations.
    unsafe {
        buffer.copy_from(&[value], 0, 1);
    }
    model.push(value);
}

/// Appends one modeled value through spare storage plus `advance`.
fn append_through_spare(
    buffer: &mut Buffer<u8>,
    model: &mut Vec<u8>,
    value: u8,
) {
    ensure_spare(buffer);
    buffer.spare_mut()[0] = value;
    // SAFETY: `ensure_spare` guarantees one initialized spare element.
    unsafe {
        buffer.advance(1);
    }
    model.push(value);
}

/// Copies and consumes a modeled readable prefix.
fn copy_out(buffer: &mut Buffer<u8>, model: &mut Vec<u8>, argument: u8) {
    let count = usize::from(argument) % (model.len() + 1);
    let mut output = vec![0_u8; count + 2];
    // SAFETY: `count` is bounded by the readable model and the destination
    // range `1..1 + count` is valid.
    unsafe {
        buffer.copy_to(&mut output, 1, count);
    }
    assert_eq!(&model[..count], &output[1..1 + count]);
    model.drain(..count);
}

/// Consumes a modeled readable prefix without copying it.
fn consume(buffer: &mut Buffer<u8>, model: &mut Vec<u8>, argument: u8) {
    let count = usize::from(argument) % (model.len() + 1);
    // SAFETY: `count` is bounded by the current readable model length.
    unsafe {
        buffer.consume(count);
    }
    model.drain(..count);
}

/// Verifies the readable-window model and all public cursor invariants.
fn assert_invariants(buffer: &Buffer<u8>, model: &[u8]) {
    assert!(buffer.position() <= buffer.limit());
    assert!(buffer.limit() <= buffer.capacity());
    assert_eq!(buffer.available(), model.len());
    assert_eq!(buffer.readable(), model);
    assert_eq!(buffer.capacity() - buffer.limit(), buffer.spare_capacity());
    assert_eq!(buffer.is_empty(), model.is_empty());
    assert_eq!(buffer.is_full(), buffer.spare_capacity() == 0);
}
