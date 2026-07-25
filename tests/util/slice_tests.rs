// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use qubit_io::UncheckedSlice;

#[test]
fn test_read_unchecked_reads_value() {
    let input = [1_u8, 2, 3];
    assert_eq!(unsafe { qubit_io::UncheckedSlice::read(&input, 1) }, 2);
}

#[test]
fn test_write_unchecked_writes_value() {
    let mut output = [1_u8, 2, 3];
    unsafe { qubit_io::UncheckedSlice::write(&mut output, 1, 9) };
    assert_eq!(output, [1, 9, 3]);
}

#[test]
fn test_write_unchecked_moves_non_copy_value() {
    let mut output = [
        String::from("left"),
        String::from("middle"),
        String::from("right"),
    ];
    unsafe {
        qubit_io::UncheckedSlice::write(&mut output, 1, String::from("updated"))
    };
    assert_eq!(output[1], "updated");
}

#[test]
fn test_ref_unchecked_returns_reference() {
    let input = [4_u16, 5, 6];
    assert_eq!(unsafe { *qubit_io::UncheckedSlice::get(&input, 2) }, 6);
}

#[test]
fn test_mut_unchecked_writes_reference() {
    let mut output = [10_u32, 20, 30];
    unsafe {
        *qubit_io::UncheckedSlice::get_mut(&mut output, 0) = 12_345;
    }
    assert_eq!(output[0], 12_345);
}

#[test]
fn test_range_fits_checks_range() {
    assert!(UncheckedSlice::range_fits(8, 2, 6));
    assert!(!UncheckedSlice::range_fits(8, 3, 6));
}

#[test]
fn test_range_end_returns_exclusive_end_index() {
    assert_eq!(UncheckedSlice::range_end(8, 2, 6), Some(8));
    assert_eq!(UncheckedSlice::range_end(8, 3, 6), None);
    assert_eq!(UncheckedSlice::range_end(8, usize::MAX, 1), None);
}

#[test]
fn test_checked_range_end_returns_io_error() {
    let error =
        UncheckedSlice::checked_range_end(8, 3, 6, "range exceeds buffer")
            .expect_err("invalid range should return an I/O error");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "range exceeds buffer");
}

#[test]
fn test_index_saturating_add_checks_overflow() {
    assert_eq!(10usize.saturating_add(2), 12);
    assert_eq!(usize::MAX.saturating_add(1), usize::MAX);
}

#[test]
fn test_ne_unaligned_unchecked_reads_and_writes() {
    let mut output = [0_u8; 8];
    // SAFETY: Writes a little-endian u16 to valid unaligned offset 1.
    unsafe {
        qubit_io::UncheckedSlice::write_ne_unaligned(
            &mut output,
            1,
            0x1234_u16,
        );
        let value = UncheckedSlice::read_ne_unaligned::<u16>(&output, 1);
        assert_eq!(value, 0x1234_u16);
    }
    assert_eq!(output[1], 0x34);
    assert_eq!(output[2], 0x12);
}

#[test]
fn test_subslice_returns_range() {
    let input = [1_u8, 2, 3, 4, 5];
    let slice = unsafe { qubit_io::UncheckedSlice::subslice(&input, 1, 3) };
    assert_eq!(slice, &[2, 3, 4]);
}

#[test]
fn test_subslice_mut_returns_mutable_range() {
    let mut output = [1_u8, 2, 3, 4, 5];
    let slice =
        unsafe { qubit_io::UncheckedSlice::subslice_mut(&mut output, 2, 2) };
    slice.copy_from_slice(&[8, 9]);
    assert_eq!(output, [1, 2, 8, 9, 5]);
}

#[test]
fn test_copy_nonoverlapping_unchecked_copies_slice() {
    let source = [1_u8, 2, 3, 4];
    let mut destination = [0_u8, 0, 0, 0];
    unsafe {
        qubit_io::UncheckedSlice::copy_nonoverlapping(
            &source,
            0,
            &mut destination,
            0,
            4,
        );
    }
    assert_eq!(destination, source);
}

#[test]
fn test_copy_unchecked_copies_overlapping_range() {
    let mut buffer = [0_u8, 1, 2, 3, 4, 5, 6, 7];
    unsafe {
        UncheckedSlice::copy_within(&mut buffer, 2, 0, 4);
    }
    assert_eq!(buffer, [2, 3, 4, 5, 4, 5, 6, 7]);
}
