// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::Buffer;

#[test]
fn test_window_accessors_expose_consumed_readable_and_spare() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    assert!(buffer.consumed().is_empty());
    assert!(buffer.readable().is_empty());
    assert_eq!(6, buffer.spare().len());
    let spare_snapshot = buffer.spare().to_vec();
    let spare_mut = buffer.spare_mut();
    assert_eq!(&spare_snapshot, &*spare_mut);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from(b"abcde", 0, 5);
    }
    // SAFETY: Five readable bytes were appended above.
    unsafe {
        buffer.consume(2);
    }

    assert_eq!(b"ab", buffer.consumed());
    assert_eq!(b"cde", buffer.readable());
    assert_eq!(1, buffer.spare().len());
    assert_eq!(buffer.spare(), &buffer.data()[buffer.limit()..]);
}

#[test]
fn test_readable_returns_position_to_limit_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from(b"abcde", 0, 5);
    }
    // SAFETY: Five readable bytes were appended above.
    unsafe {
        buffer.consume(2);
    }

    assert_eq!(b"cde", buffer.readable());
    assert_eq!(3, buffer.readable().len());
    assert_eq!(buffer.available(), buffer.readable().len());
}

#[test]
fn test_with_capacity_initializes_empty_window() {
    let buffer = Buffer::<u8>::with_capacity(4);

    assert_eq!(4, buffer.capacity());
    assert_eq!(0, buffer.position());
    assert_eq!(0, buffer.limit());
    assert_eq!(0, buffer.available());
    assert_eq!(4, buffer.spare_capacity());
    assert_eq!(4, buffer.spare().len());
    assert!(buffer.is_empty());
    assert!(!buffer.is_full());
}

#[test]
fn test_copy_from_appends_to_spare_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);
    let input = b"abcdef";

    // SAFETY: `input[1..4]` is valid and the empty buffer has enough spare
    // capacity for three bytes.
    unsafe {
        buffer.copy_from(input, 1, 3);
    }

    assert_eq!(0, buffer.position());
    assert_eq!(3, buffer.limit());
    assert_eq!(3, buffer.available());
    assert_eq!(b"bcd", buffer.readable());
}

#[test]
fn test_advance_marks_spare_values_as_readable() {
    let mut buffer = Buffer::<u8>::with_capacity(4);

    buffer.spare_mut()[0..2].copy_from_slice(b"ab");
    // SAFETY: Two initialized bytes are available in the spare window.
    unsafe {
        buffer.advance(2);
    }

    assert_eq!(0, buffer.position());
    assert_eq!(2, buffer.limit());
    assert_eq!(2, buffer.available());
    assert_eq!(2, buffer.spare_capacity());
    assert_eq!(b"ab", buffer.readable());
}

#[test]
fn test_copy_to_consumes_from_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);
    let input = b"abcdef";
    let mut output = [0_u8; 5];

    // SAFETY: The input range and spare range are valid for six bytes.
    unsafe {
        buffer.copy_from(input, 0, 6);
    }
    // SAFETY: Six readable bytes were appended above.
    unsafe {
        buffer.consume(2);
    }
    // SAFETY: Three bytes are available and `output[1..4]` is valid.
    unsafe {
        buffer.copy_to(&mut output, 1, 3);
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
        buffer.copy_from(b"abcde", 0, 5);
    }
    // SAFETY: Five readable bytes were appended above.
    unsafe {
        buffer.consume(2);
    }
    buffer.compact();

    assert_eq!(0, buffer.position());
    assert_eq!(3, buffer.limit());
    assert_eq!(3, buffer.available());
    assert_eq!(3, buffer.spare_capacity());
    assert_eq!(b"cde", buffer.readable());
}

#[test]
fn test_position_limit_and_available_describe_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for five bytes.
    unsafe {
        buffer.copy_from(b"abcde", 0, 5);
    }
    // SAFETY: Five readable bytes were appended above.
    unsafe {
        buffer.consume(2);
    }

    let start = buffer.position();
    let end = buffer.limit();

    assert_eq!(2, start);
    assert_eq!(5, end);
    assert_eq!(3, buffer.available());
    assert_eq!(b"cde", buffer.readable());
}

#[test]
fn test_spare_raw_parts_mut_exposes_backing_buffer_index_and_count() {
    let mut buffer = Buffer::<u8>::with_capacity(6);

    // SAFETY: The input range and spare range are valid for two bytes.
    unsafe {
        buffer.copy_from(b"ab", 0, 2);
    }

    {
        let (data, index, count) = buffer.spare_raw_parts_mut();

        assert_eq!(2, index);
        assert_eq!(4, count);
        data[index..index + 2].copy_from_slice(b"cd");
    }
    assert_eq!(b"cd", &buffer.spare()[0..2]);
    // SAFETY: Two bytes were initialized in the spare window above.
    unsafe {
        buffer.advance(2);
    }

    assert_eq!(b"abcd", buffer.readable());
}

#[test]
fn test_compact_clears_empty_readable_window() {
    let mut buffer = Buffer::<u8>::with_capacity(4);

    // SAFETY: Three default-initialized spare bytes fit in the buffer.
    unsafe {
        buffer.advance(3);
    }
    // SAFETY: Three readable bytes were made available above.
    unsafe {
        buffer.consume(3);
    }
    buffer.compact();

    assert_eq!(0, buffer.position());
    assert_eq!(0, buffer.limit());
    assert_eq!(0, buffer.available());
    assert_eq!(4, buffer.spare_capacity());
    assert!(buffer.is_empty());
}
