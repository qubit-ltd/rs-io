// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::Buffer;

#[test]
fn test_with_capacity_initializes_empty_window() {
    let buffer = Buffer::<u8>::with_capacity(4);

    assert_eq!(4, buffer.capacity());
    assert_eq!(0, buffer.position());
    assert_eq!(0, buffer.limit());
    assert_eq!(0, buffer.available());
    assert_eq!(4, buffer.spare_capacity());
    assert!(buffer.is_empty());
    assert!(!buffer.is_full());
}

#[test]
fn test_copy_from_unchecked_appends_to_spare_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);
    let input = b"abcdef";

    // SAFETY: `input[1..4]` is valid and the empty buffer has enough spare
    // capacity for three bytes.
    unsafe {
        buffer.copy_from_unchecked(input, 1, 3);
    }

    assert_eq!(0, buffer.position());
    assert_eq!(3, buffer.limit());
    assert_eq!(3, buffer.available());
    assert_eq!(b"bcd", &buffer.data()[0..3]);
}

#[test]
fn test_advance_marks_spare_values_as_readable() {
    let mut buffer = Buffer::<u8>::with_capacity(4);

    buffer.data_mut()[0..2].copy_from_slice(b"ab");
    buffer.advance(2);

    assert_eq!(0, buffer.position());
    assert_eq!(2, buffer.limit());
    assert_eq!(2, buffer.available());
    assert_eq!(2, buffer.spare_capacity());
    assert_eq!(b"ab", &buffer.data()[0..2]);
}

#[test]
fn test_copy_to_unchecked_consumes_from_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);
    let input = b"abcdef";
    let mut output = [0_u8; 5];

    // SAFETY: The input range and spare range are valid for six bytes.
    unsafe {
        buffer.copy_from_unchecked(input, 0, 6);
    }
    buffer.consume(2);
    // SAFETY: Three bytes are available and `output[1..4]` is valid.
    unsafe {
        buffer.copy_to_unchecked(&mut output, 1, 3);
    }

    assert_eq!([0, b'c', b'd', b'e', 0], output);
    assert_eq!(5, buffer.position());
    assert_eq!(6, buffer.limit());
    assert_eq!(1, buffer.available());
}

#[test]
fn test_compact_moves_unread_tail_to_front() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from_unchecked(b"abcde", 0, 5);
    }
    buffer.consume(2);
    buffer.compact();

    assert_eq!(0, buffer.position());
    assert_eq!(3, buffer.limit());
    assert_eq!(3, buffer.available());
    assert_eq!(3, buffer.spare_capacity());
    assert_eq!(b"cde", &buffer.data()[0..3]);
}

#[test]
fn test_unread_slice_returns_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from_unchecked(b"abcde", 0, 5);
    }
    buffer.consume(2);

    assert_eq!(b"cde", buffer.unread_slice());
}

#[test]
fn test_unread_raw_parts_exposes_backing_buffer_index_and_count() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from_unchecked(b"abcde", 0, 5);
    }
    buffer.consume(2);

    let (data, index, count) = buffer.unread_raw_parts();

    assert_eq!(2, index);
    assert_eq!(3, count);
    assert_eq!(b"cde", &data[index..index + count]);
}

#[test]
fn test_compact_clears_empty_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(4);

    buffer.advance(3);
    buffer.consume(3);
    buffer.compact();

    assert_eq!(0, buffer.position());
    assert_eq!(0, buffer.limit());
    assert_eq!(0, buffer.available());
    assert_eq!(4, buffer.spare_capacity());
    assert!(buffer.is_empty());
}
