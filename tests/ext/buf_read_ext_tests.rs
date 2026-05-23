/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{
    BufRead,
    Cursor,
    ErrorKind,
};

use qubit_io::BufReadExt;

#[test]
fn test_read_until_limited_reads_through_delimiter() {
    let mut input = Cursor::new(b"abc,def".to_vec());

    let value = input
        .read_until_limited(b',', 4)
        .expect("delimited bytes should be read");

    assert_eq!(b"abc,", value.as_slice());
    assert_eq!(
        b"def",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_until_limited_accepts_large_limit() {
    let mut input = Cursor::new(b"abc,def".to_vec());

    let value = input
        .read_until_limited(b',', 9000)
        .expect("delimited bytes should be read");

    assert_eq!(b"abc,", value.as_slice());
}

#[test]
fn test_read_until_limited_accepts_eof_before_delimiter() {
    let mut input = Cursor::new(b"abc".to_vec());

    let value = input
        .read_until_limited(b',', 3)
        .expect("EOF within the limit should be accepted");

    assert_eq!(b"abc", value.as_slice());
}

#[test]
fn test_read_until_limited_into_appends_through_delimiter() {
    let mut input = Cursor::new(b"abc,def".to_vec());
    let mut output = b"prefix-".to_vec();

    let count = input
        .read_until_limited_into(b',', &mut output, 4)
        .expect("delimited bytes should be appended");

    assert_eq!(4, count);
    assert_eq!(b"prefix-abc,", output.as_slice());
    assert_eq!(
        b"def",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_until_limited_rejects_input_beyond_limit() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());

    let error = input
        .read_until_limited(b'\n', 3)
        .expect_err("input beyond the limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "input exceeds maximum length of 3 bytes before delimiter 10",
        error.to_string()
    );
    assert_eq!(
        b"def\n",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_until_limited_into_rejects_input_beyond_limit_after_prefix() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());
    let mut output = b"prefix-".to_vec();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 3)
        .expect_err("input beyond the limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix-abc", output.as_slice());
    assert_eq!(
        b"def\n",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_until_limited_into_rejects_zero_limit_without_appending() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());
    let mut output = b"prefix-".to_vec();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 0)
        .expect_err("input beyond the zero limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix-", output.as_slice());
    assert_eq!(
        b"abcdef\n",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_line_limited_reads_utf8_line() {
    let mut input = Cursor::new("hello 世界\nnext".as_bytes().to_vec());

    let value = input
        .read_line_limited(16)
        .expect("line should be read within the limit");

    assert_eq!("hello 世界\n", value);
    assert_eq!(
        b"next",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_line_limited_into_appends_utf8_line() {
    let mut input = Cursor::new("hello 世界\nnext".as_bytes().to_vec());
    let mut output = String::from("prefix-");

    let count = input
        .read_line_limited_into(&mut output, 16)
        .expect("line should be appended within the limit");

    assert_eq!("hello 世界\n".len(), count);
    assert_eq!("prefix-hello 世界\n", output);
    assert_eq!(
        b"next",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_read_line_limited_into_accepts_large_limit() {
    let mut input = Cursor::new("hello\nnext".as_bytes().to_vec());
    let mut output = String::new();

    let count = input
        .read_line_limited_into(&mut output, 9000)
        .expect("line should be appended within the limit");

    assert_eq!("hello\n".len(), count);
    assert_eq!("hello\n", output);
}

#[test]
fn test_read_line_limited_rejects_invalid_utf8() {
    let mut input = Cursor::new(vec![0xff, b'\n']);

    let error = input
        .read_line_limited(8)
        .expect_err("invalid UTF-8 line should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("limited line is not valid UTF-8")
    );
}

#[test]
fn test_read_line_limited_into_rejects_invalid_utf8_without_appending() {
    let mut input = Cursor::new(vec![0xff, b'\n']);
    let mut output = String::from("prefix");

    let error = input
        .read_line_limited_into(&mut output, 8)
        .expect_err("invalid UTF-8 line should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("limited line is not valid UTF-8")
    );
    assert_eq!("prefix", output);
}

#[test]
fn test_discard_until_limited_discards_through_delimiter() {
    let mut input = Cursor::new(b"abc,def".to_vec());

    let count = input
        .discard_until_limited(b',', 4)
        .expect("bytes should be discarded through delimiter");

    assert_eq!(4, count);
    assert_eq!(
        b"def",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_discard_until_limited_accepts_eof_before_delimiter() {
    let mut input = Cursor::new(b"abc".to_vec());

    let count = input
        .discard_until_limited(b',', 3)
        .expect("EOF within the limit should be accepted while discarding");

    assert_eq!(3, count);
    assert!(
        input
            .fill_buf()
            .expect("input should be exhausted")
            .is_empty()
    );
}

#[test]
fn test_discard_until_limited_rejects_input_beyond_limit() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());

    let error = input
        .discard_until_limited(b'\n', 3)
        .expect_err("input beyond the limit should be rejected while discarding");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        b"def\n",
        input.fill_buf().expect("remaining bytes should exist")
    );
}

#[test]
fn test_discard_until_limited_rejects_zero_limit_without_consuming() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());

    let error = input
        .discard_until_limited(b'\n', 0)
        .expect_err("input beyond the zero limit should be rejected while discarding");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        b"abcdef\n",
        input.fill_buf().expect("remaining bytes should exist")
    );
}
