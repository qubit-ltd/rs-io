// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::BufRead;
use std::io::BufReader;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;

use qubit_io::std_io::ext::BufReadExt;

struct InterruptingReader {
    bytes: Cursor<Vec<u8>>,
    calls: usize,
    interrupts: Vec<usize>,
}

impl Read for InterruptingReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.calls += 1;
        if self.interrupts.contains(&self.calls) {
            return Err(ErrorKind::Interrupted.into());
        }
        self.bytes.read(output)
    }
}

fn interrupted_reader(interrupts: Vec<usize>) -> BufReader<InterruptingReader> {
    BufReader::with_capacity(
        2,
        InterruptingReader {
            bytes: Cursor::new(b"abc\n".to_vec()),
            calls: 0,
            interrupts,
        },
    )
}

// Emits one Other error on the selected underlying read call.
struct ErrorReader {
    bytes: Cursor<Vec<u8>>,
    calls: usize,
    error_at: usize,
}

impl Read for ErrorReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.calls += 1;
        if self.calls == self.error_at {
            return Err(ErrorKind::Other.into());
        }
        self.bytes.read(output)
    }
}

#[test]
fn test_read_until_limited_reads_through_delimiter() {
    let mut input = Cursor::new(b"abc,def".to_vec());

    let value = input
        .read_until_limited(b',', 4)
        .expect("delimited bytes should be read");

    assert_eq!(b"abc,", value.as_slice());
    assert_eq!(b"def", input.fill_buf().expect("remaining bytes should exist"));
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
    assert_eq!(b"def", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_read_until_limited_retries_interrupted() {
    let mut reader = interrupted_reader(vec![1]);

    assert_eq!(
        b"abc\n",
        reader
            .read_until_limited(b'\n', 4)
            .expect("line should be read after retrying an interrupted read")
            .as_slice()
    );
}

#[test]
fn test_read_until_limited_into_retries_interrupted_after_progress() {
    let mut reader = interrupted_reader(vec![2]);
    let mut output = b"prefix-".to_vec();
    assert_eq!(
        4,
        reader
            .read_until_limited_into(b'\n', &mut output, 4)
            .expect("line should be appended after retrying an interrupted read")
    );
    assert_eq!(b"prefix-abc\n", output.as_slice());
}

#[test]
fn test_read_line_limited_retries_interrupted() {
    let mut reader = interrupted_reader(vec![1, 2]);

    assert_eq!(
        "abc\n",
        reader
            .read_line_limited(4)
            .expect("line should be read after retrying interrupted reads")
    );
}

#[test]
fn test_read_line_limited_into_retries_interrupted_after_progress() {
    let mut reader = interrupted_reader(vec![2]);
    let mut output = String::from("prefix-");

    assert_eq!(
        4,
        reader
            .read_line_limited_into(&mut output, 4)
            .expect("line should be appended after retrying an interrupted read")
    );
    assert_eq!("prefix-abc\n", output);
}

#[test]
fn test_discard_until_limited_retries_interrupted_after_progress() {
    let mut reader = interrupted_reader(vec![2]);

    assert_eq!(
        4,
        reader
            .discard_until_limited(b'\n', 4)
            .expect("line should be discarded after retrying an interrupted read")
    );
    assert!(
        reader
            .fill_buf()
            .expect("buffer should be readable after discarding the line")
            .is_empty(),
        "buffer should be empty after discarding through the delimiter"
    );
}

#[test]
fn test_read_until_limited_into_preserves_limit_behavior_after_interrupted() {
    let mut reader = interrupted_reader(vec![2]);
    let mut output = b"prefix-".to_vec();

    let error = reader.read_until_limited_into(b'\n', &mut output, 3).unwrap_err();

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix-", output.as_slice());
    assert_eq!(
        b"\n",
        reader
            .fill_buf()
            .expect("buffer should be readable after the limit error")
    );
}

#[test]
fn test_discard_until_limited_preserves_limit_behavior_after_interrupted() {
    let mut reader = interrupted_reader(vec![2]);

    let error = reader.discard_until_limited(b'\n', 3).unwrap_err();

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        b"\n",
        reader
            .fill_buf()
            .expect("buffer should be readable after the limit error")
    );
}

#[test]
fn test_read_until_limited_into_restores_output_on_other_error_after_progress() {
    let mut reader = BufReader::with_capacity(
        2,
        ErrorReader {
            bytes: Cursor::new(b"abc\n".to_vec()),
            calls: 0,
            error_at: 2,
        },
    );
    let mut output = b"prefix-".to_vec();

    let error = reader.read_until_limited_into(b'\n', &mut output, 4).unwrap_err();

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(b"prefix-", output.as_slice());
    assert_eq!(
        b"c\n",
        reader
            .fill_buf()
            .expect("buffer should be readable after the underlying error")
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
    assert_eq!(b"def\n", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_read_until_limited_into_rejects_input_beyond_limit_after_prefix() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());
    let mut output = b"prefix-".to_vec();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 3)
        .expect_err("input beyond the limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix-", output.as_slice());
    assert_eq!(b"def\n", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_read_until_limited_into_rejects_oversized_input_after_partial_reads() {
    let mut input = Cursor::new(b"abcd\n".to_vec());
    let mut output = Vec::new();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 3)
        .expect_err("input beyond the limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(output.is_empty(), "output should remain unchanged on error");
}

#[test]
fn test_read_line_limited_into_rejects_oversized_input_after_prefix() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());
    let mut output = String::from("prefix-");

    let error = input
        .read_line_limited_into(&mut output, 3)
        .expect_err("oversized line should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!("prefix-", output);
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
    assert_eq!(b"abcdef\n", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_read_line_limited_reads_utf8_line() {
    let mut input = Cursor::new("hello 世界\nnext".as_bytes().to_vec());

    let value = input
        .read_line_limited(16)
        .expect("line should be read within the limit");

    assert_eq!("hello 世界\n", value);
    assert_eq!(b"next", input.fill_buf().expect("remaining bytes should exist"));
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
    assert_eq!(b"next", input.fill_buf().expect("remaining bytes should exist"));
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
        error.to_string().starts_with("limited line is not valid UTF-8"),
        "error should identify the invalid UTF-8 line"
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
        error.to_string().starts_with("limited line is not valid UTF-8"),
        "error should identify the invalid UTF-8 line"
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
    assert_eq!(b"def", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_discard_until_limited_accepts_eof_before_delimiter() {
    let mut input = Cursor::new(b"abc".to_vec());

    let count = input
        .discard_until_limited(b',', 3)
        .expect("EOF within the limit should be accepted while discarding");

    assert_eq!(3, count);
    assert!(input.fill_buf().expect("input should be exhausted").is_empty());
}

#[test]
fn test_discard_until_limited_rejects_input_beyond_limit() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());

    let error = input
        .discard_until_limited(b'\n', 3)
        .expect_err("input beyond the limit should be rejected while discarding");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"def\n", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_discard_until_limited_rejects_zero_limit_without_consuming() {
    let mut input = Cursor::new(b"abcdef\n".to_vec());

    let error = input
        .discard_until_limited(b'\n', 0)
        .expect_err("input beyond the zero limit should be rejected while discarding");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"abcdef\n", input.fill_buf().expect("remaining bytes should exist"));
}

#[test]
fn test_limited_delimiter_boundaries_preserve_consumption_and_rollback() {
    struct Case<'a> {
        bytes: &'a [u8],
        limit: usize,
        expected: Option<&'a [u8]>,
        position: u64,
    }

    let cases = [
        Case {
            bytes: b"",
            limit: 0,
            expected: Some(b""),
            position: 0,
        },
        Case {
            bytes: b"x",
            limit: 0,
            expected: None,
            position: 0,
        },
        Case {
            bytes: b"a\n",
            limit: 2,
            expected: Some(b"a\n"),
            position: 2,
        },
        Case {
            bytes: b"ab\n",
            limit: 2,
            expected: None,
            position: 2,
        },
        Case {
            bytes: b"ab",
            limit: 2,
            expected: Some(b"ab"),
            position: 2,
        },
    ];

    for Case {
        bytes,
        limit,
        expected,
        position,
    } in cases
    {
        let mut input = Cursor::new(bytes.to_vec());
        let mut output = b"prefix".to_vec();
        let result = input.read_until_limited_into(b'\n', &mut output, limit);
        assert_eq!(expected.is_none(), result.is_err(), "bytes={bytes:?}, limit={limit}");
        assert_eq!(position, input.position());
        if let Some(expected) = expected {
            assert_eq!(expected, &output[b"prefix".len()..]);
        } else {
            assert_eq!(b"prefix", output.as_slice());
            assert_eq!(
                ErrorKind::InvalidData,
                result.expect_err("limit should be exceeded").kind()
            );
        }

        let mut input = Cursor::new(bytes.to_vec());
        let result = input.discard_until_limited(b'\n', limit);
        assert_eq!(expected.is_none(), result.is_err(), "bytes={bytes:?}, limit={limit}");
        assert_eq!(position, input.position());
        if expected.is_none() {
            assert_eq!(
                ErrorKind::InvalidData,
                result.expect_err("limit should be exceeded").kind()
            );
        }
    }
}

#[test]
fn test_limited_delimiter_rejects_large_buffer_after_consuming_only_limit() {
    let bytes = vec![b'x'; 1024 * 1024];
    let mut input = Cursor::new(bytes.clone());
    let mut output = b"prefix".to_vec();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 1)
        .expect_err("input beyond the one byte limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix", output.as_slice());
    assert_eq!(1, input.position());

    let mut input = Cursor::new(bytes);
    let error = input
        .discard_until_limited(b'\n', 1)
        .expect_err("input beyond the one byte limit should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(1, input.position());
}

#[test]
fn test_limited_delimiter_handles_small_buffer_reader_chunks() {
    let mut input = BufReader::with_capacity(2, Cursor::new(b"ab\nrest".to_vec()));
    let mut output = b"prefix".to_vec();

    let error = input
        .read_until_limited_into(b'\n', &mut output, 2)
        .expect_err("delimiter beyond the limit should be rejected across chunks");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"prefix", output.as_slice());
    assert_eq!(b"\nr", input.fill_buf().expect("buffer should retain unread bytes"));

    let mut input = BufReader::with_capacity(2, Cursor::new(b"ab\nrest".to_vec()));
    let error = input
        .discard_until_limited(b'\n', 2)
        .expect_err("delimiter beyond the limit should be rejected across chunks");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(b"\nr", input.fill_buf().expect("buffer should retain unread bytes"));
}
