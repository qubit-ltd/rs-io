// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_utils::{
    SliceRange,
    UncheckedSlice,
};

/// Bounds allocations even when the target is invoked without CI flags.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_ranges(data);
    fuzz_elements(data);
    fuzz_copies(data);
    fuzz_unaligned_values(data);
});

/// Checks range helpers against checked standard-library arithmetic.
fn fuzz_ranges(data: &[u8]) {
    let len = fuzz_usize(data, 0);
    let start = fuzz_usize(data, 1);
    let count = fuzz_usize(data, 2);
    let expected = start.checked_add(count).filter(|&end| end <= len);

    assert_eq!(expected, SliceRange::range_end(len, start, count));
    assert_eq!(
        expected.is_some(),
        SliceRange::range_fits(len, start, count)
    );
}

/// Checks indexed element and subslice operations against safe indexing.
fn fuzz_elements(data: &[u8]) {
    let fallback = [0_u8];
    let input = if data.is_empty() { &fallback } else { data };
    let index = fuzz_usize(data, 3) % input.len();
    let start = fuzz_usize(data, 4) % (input.len() + 1);
    let count = fuzz_usize(data, 5) % (input.len() - start + 1);

    // SAFETY: `index` is reduced modulo the non-empty input length.
    assert_eq!(input[index], unsafe { UncheckedSlice::read(input, index) });
    // SAFETY: `index` is reduced modulo the non-empty input length.
    assert_eq!(&input[index], unsafe { UncheckedSlice::get(input, index) });
    // SAFETY: `start` and `count` were constrained to a valid input range.
    assert_eq!(&input[start..start + count], unsafe {
        UncheckedSlice::subslice(input, start, count)
    });

    let replacement = data.first().copied().unwrap_or_default();
    let mut actual = input.to_vec();
    let mut expected = actual.clone();
    expected[index] = replacement;
    // SAFETY: `index` is valid for both equally sized buffers.
    unsafe {
        UncheckedSlice::write(&mut actual, index, replacement);
    }
    assert_eq!(expected, actual);

    let replacement = replacement.wrapping_add(1);
    expected[index] = replacement;
    // SAFETY: `index` is valid for both equally sized buffers.
    unsafe {
        *UncheckedSlice::get_mut(&mut actual, index) = replacement;
    }
    assert_eq!(expected, actual);

    // SAFETY: `start` and `count` were constrained to a valid mutable range.
    let actual_range =
        unsafe { UncheckedSlice::subslice_mut(&mut actual, start, count) };
    assert_eq!(&expected[start..start + count], actual_range);
}

/// Checks overlapping and non-overlapping copies against safe slice methods.
fn fuzz_copies(data: &[u8]) {
    let fallback = [0_u8];
    let source = if data.is_empty() { &fallback } else { data };
    let count = fuzz_usize(data, 6) % (source.len() + 1);
    let source_index = fuzz_usize(data, 7) % (source.len() - count + 1);
    let mut destination = vec![0_u8; source.len() + 1];
    let destination_index =
        fuzz_usize(data, 8) % (destination.len() - count + 1);
    let mut expected = destination.clone();
    expected[destination_index..destination_index + count]
        .copy_from_slice(&source[source_index..source_index + count]);

    // SAFETY: Both ranges were constrained to valid, separate allocations.
    unsafe {
        UncheckedSlice::copy_nonoverlapping(
            source,
            source_index,
            &mut destination,
            destination_index,
            count,
        );
    }
    assert_eq!(expected, destination);

    let mut actual = source.to_vec();
    let mut expected = actual.clone();
    let count = fuzz_usize(data, 9) % (actual.len() + 1);
    let source_index = fuzz_usize(data, 10) % (actual.len() - count + 1);
    let destination_index = fuzz_usize(data, 11) % (actual.len() - count + 1);
    expected.copy_within(source_index..source_index + count, destination_index);
    // SAFETY: Both ranges were constrained to the same valid allocation;
    // overlapping ranges are allowed by `copy_within`.
    unsafe {
        UncheckedSlice::copy_within(
            &mut actual,
            source_index,
            destination_index,
            count,
        );
    }
    assert_eq!(expected, actual);
}

/// Checks unaligned integer access against native-endian byte conversion.
fn fuzz_unaligned_values(data: &[u8]) {
    if data.len() < size_of::<u32>() {
        return;
    }
    let index = fuzz_usize(data, 12) % (data.len() - size_of::<u32>() + 1);
    let expected = u32::from_ne_bytes(
        data[index..index + size_of::<u32>()]
            .try_into()
            .expect("the constrained range contains four bytes"),
    );
    // SAFETY: The selected range contains a valid initialized `u32` byte
    // representation; every bit pattern is valid for `u32`.
    let actual =
        unsafe { UncheckedSlice::read_ne_unaligned::<u32>(data, index) };
    assert_eq!(expected, actual);

    let mut output = vec![0_u8; data.len()];
    // SAFETY: The selected range fits in `output`; `u32` has no invalid bit
    // patterns or uninitialized padding.
    unsafe {
        UncheckedSlice::write_ne_unaligned(&mut output, index, actual);
    }
    assert_eq!(
        actual.to_ne_bytes(),
        output[index..index + size_of::<u32>()]
    );
}

/// Derives a `usize` while deliberately retaining overflow-heavy inputs.
fn fuzz_usize(data: &[u8], offset: usize) -> usize {
    let mut value = 0_usize;
    for index in 0..size_of::<usize>() {
        let byte = data
            .get(offset.wrapping_add(index))
            .copied()
            .unwrap_or_default();
        value = value.rotate_left(8) ^ usize::from(byte);
    }
    if data.get(offset).is_some_and(|byte| byte & 0x80 != 0) {
        usize::MAX - value
    } else {
        value
    }
}
