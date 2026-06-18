// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
use qubit_io::{
    copy_unchecked,
    range_fits,
};

#[test]
fn read_unchecked_reads_value() {
    let input = [1_u8, 2, 3];
    assert_eq!(unsafe { qubit_io::read_unchecked(&input, 1) }, 2);
}

#[test]
fn write_unchecked_writes_value() {
    let mut output = [1_u8, 2, 3];
    unsafe { qubit_io::write_unchecked(&mut output, 1, 9) };
    assert_eq!(output, [1, 9, 3]);
}

#[test]
fn ref_unchecked_returns_reference() {
    let input = [4_u16, 5, 6];
    assert_eq!(unsafe { *qubit_io::ref_unchecked(&input, 2) }, 6);
}

#[test]
fn mut_unchecked_writes_reference() {
    let mut output = [10_u32, 20, 30];
    unsafe {
        *qubit_io::mut_unchecked(&mut output, 0) = 12_345;
    }
    assert_eq!(output[0], 12_345);
}

#[test]
fn range_fits_checks_range() {
    assert!(range_fits(8, 2, 6));
    assert!(!range_fits(8, 3, 6));
}

#[test]
fn index_saturating_add_checks_overflow() {
    assert_eq!(10usize.saturating_add(2), 12);
    assert_eq!(usize::MAX.saturating_add(1), usize::MAX);
}

#[test]
fn ne_unaligned_unchecked_reads_and_writes() {
    let mut output = [0_u8; 8];
    // SAFETY: Writes a little-endian u16 to valid unaligned offset 1.
    unsafe {
        qubit_io::write_ne_unaligned_unchecked(&mut output, 1, 0x1234_u16);
        let value = qubit_io::read_ne_unaligned_unchecked::<u16>(&output, 1);
        assert_eq!(value, 0x1234_u16);
    }
    assert_eq!(output[1], 0x34);
    assert_eq!(output[2], 0x12);
}

#[test]
fn copy_nonoverlapping_unchecked_copies_slice() {
    let source = [1_u8, 2, 3, 4];
    let mut destination = [0_u8, 0, 0, 0];
    unsafe {
        qubit_io::copy_nonoverlapping_unchecked(
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
fn copy_unchecked_copies_overlapping_range() {
    let mut buffer = [0_u8, 1, 2, 3, 4, 5, 6, 7];
    unsafe {
        copy_unchecked(&mut buffer, 2, 0, 4);
    }
    assert_eq!(buffer, [2, 3, 4, 5, 4, 5, 6, 7]);
}
