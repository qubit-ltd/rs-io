// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
};

use qubit_io::ReadExt;

struct ShortReader {
    data: Vec<u8>,
    position: usize,
    max_chunk: usize,
}

impl ShortReader {
    fn new(data: &[u8], max_chunk: usize) -> Self {
        Self {
            data: data.to_vec(),
            position: 0,
            max_chunk,
        }
    }
}

impl Read for ShortReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.data.len() {
            return Ok(0);
        }
        let remaining = self.data.len() - self.position;
        let count = remaining.min(buffer.len()).min(self.max_chunk);
        buffer[..count]
            .copy_from_slice(&self.data[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

struct InterruptedOnceReader {
    interrupted: bool,
    inner: ShortReader,
}

impl InterruptedOnceReader {
    fn new(data: &[u8]) -> Self {
        Self {
            interrupted: false,
            inner: ShortReader::new(data, data.len().max(1)),
        }
    }
}

impl Read for InterruptedOnceReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        self.inner.read(buffer)
    }
}

struct PanicOnRead;

impl Read for PanicOnRead {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("empty operations must not call read")
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

#[test]
fn test_read_exact_array_reads_array() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let array = reader
        .read_exact_array::<6>()
        .expect("array should be filled across short reads");

    assert_eq!(*b"abcdef", array);
}

#[test]
fn test_read_exact_array_zero_length_does_not_read() {
    let mut reader = PanicOnRead;

    let array = reader
        .read_exact_array::<0>()
        .expect("zero-length arrays should complete immediately");

    assert!(array.is_empty());
}

#[test]
fn test_read_exact_array_returns_unexpected_eof() {
    let mut reader = Cursor::new(b"ab".to_vec());

    let error = reader.read_exact_array::<3>().expect_err(
        "short input should return the standard read_exact EOF error",
    );

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_unchecked_reads_into_middle_range_once() {
    let mut reader = ShortReader::new(b"abcd", 2);
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let count = unsafe {
        reader
            .read_unchecked(&mut buffer, 2, 4)
            .expect("middle range should accept a short read")
    };

    assert_eq!(2, count);
    assert_eq!(b"xxab--yy", &buffer);
}

#[test]
fn test_read_exact_or_eof_unchecked_reads_into_middle_range() {
    let mut reader = ShortReader::new(b"abcd", 2);
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let count = unsafe {
        reader
            .read_exact_or_eof_unchecked(&mut buffer, 2, 4)
            .expect("middle range should be filled across short reads")
    };

    assert_eq!(4, count);
    assert_eq!(b"xxabcdyy", &buffer);
}

#[test]
fn test_read_exact_or_eof_unchecked_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"ab", 2);
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let count = unsafe {
        reader
            .read_exact_or_eof_unchecked(&mut buffer, 2, 4)
            .expect("EOF after partial data should not be an error")
    };

    assert_eq!(2, count);
    assert_eq!(b"xxab--yy", &buffer);
}

#[test]
fn test_read_exact_or_eof_unchecked_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abcd");
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let count = unsafe {
        reader
            .read_exact_or_eof_unchecked(&mut buffer, 2, 4)
            .expect("interrupted read should be retried")
    };

    assert_eq!(4, count);
    assert_eq!(b"xxabcdyy", &buffer);
}

#[test]
fn test_read_exact_or_eof_unchecked_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let error = unsafe {
        reader
            .read_exact_or_eof_unchecked(&mut buffer, 2, 4)
            .expect_err("non-interrupted read errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(b"xx----yy", &buffer);
}

#[test]
fn test_read_exact_unchecked_reads_into_middle_range() {
    let mut reader = ShortReader::new(b"abcd", 2);
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    unsafe {
        reader
            .read_exact_unchecked(&mut buffer, 2, 4)
            .expect("middle range should be filled across short reads");
    }

    assert_eq!(b"xxabcdyy", &buffer);
}

#[test]
fn test_read_exact_unchecked_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abcd");
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    unsafe {
        reader
            .read_exact_unchecked(&mut buffer, 2, 4)
            .expect("interrupted read should be retried");
    }

    assert_eq!(b"xxabcdyy", &buffer);
}

#[test]
fn test_read_exact_unchecked_returns_non_interrupted_error() {
    let mut reader = FailingReader;
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let error = unsafe {
        reader
            .read_exact_unchecked(&mut buffer, 2, 4)
            .expect_err("non-interrupted read errors should be returned")
    };

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!(b"xx----yy", &buffer);
}

#[test]
fn test_read_exact_unchecked_returns_unexpected_eof() {
    let mut reader = ShortReader::new(b"ab", 2);
    let mut buffer = *b"xx----yy";

    // SAFETY: `start_index..start_index + count` is `2..6`, which is within
    // `buffer` and uniquely borrowed for the duration of the read.
    let error = unsafe {
        reader
            .read_exact_unchecked(&mut buffer, 2, 4)
            .expect_err("short input should return unexpected EOF")
    };

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert_eq!(b"xxab--yy", &buffer);
}

#[test]
fn test_read_exact_vec_limited_reads_exact_length_via_trait() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let data = reader
        .read_exact_vec_limited(6, 8)
        .expect("exact vector should be filled across short reads");

    assert_eq!(b"abcdef", data.as_slice());
}

#[test]
fn test_read_exact_helpers_work_on_dyn_read_with_ufcs() {
    let mut cursor = Cursor::new(b"abcdefgh".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut prefix = [b'-'; 4];

    // SAFETY: `0..2` is within `prefix` and uniquely borrowed for the read.
    let count = unsafe {
        <dyn Read as ReadExt>::read_unchecked(reader, &mut prefix, 0, 2)
            .expect("UFCS read_unchecked should work on dyn Read")
    };
    assert_eq!(2, count);
    assert_eq!(b"ab--", &prefix);

    // SAFETY: `2..4` is within `prefix` and uniquely borrowed for the read.
    let count = unsafe {
        <dyn Read as ReadExt>::read_exact_or_eof_unchecked(
            reader,
            &mut prefix,
            2,
            2,
        )
        .expect("UFCS read_exact_or_eof_unchecked should work on dyn Read")
    };
    assert_eq!(2, count);
    assert_eq!(b"abcd", &prefix);

    let array = <dyn Read as ReadExt>::read_exact_array::<2>(reader)
        .expect("UFCS read_exact_array should work on dyn Read");
    assert_eq!(*b"ef", array);

    let data = <dyn Read as ReadExt>::read_exact_vec_limited(reader, 2, 4)
        .expect("UFCS read_exact_vec_limited should work on dyn Read");
    assert_eq!(b"gh", data.as_slice());
}

#[test]
fn test_read_exact_or_eof_delegates_to_impl_via_trait() {
    let mut reader = ShortReader::new(b"abc", 2);
    let mut buffer = [0; 3];

    let count = reader
        .read_exact_or_eof(&mut buffer)
        .expect("ReadExt should delegate to read_exact_or_eof impl");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);
}

#[test]
fn test_read_ext_ufcs_methods_work_on_dyn_read() {
    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let count = <dyn Read as ReadExt>::discard_exact_or_eof(reader, 2)
        .expect("UFCS discard_exact_or_eof should work on dyn Read");
    assert_eq!(2, count);

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count = <dyn Read as ReadExt>::copy_to(reader, &mut output)
        .expect("UFCS copy_to should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!(b"abc", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count = <dyn Read as ReadExt>::copy_to_at_most(reader, &mut output, 2)
        .expect("UFCS copy_to_at_most should work on dyn Read");
    assert_eq!(2, count);
    assert_eq!(b"ab", output.as_slice());

    let mut cursor = Cursor::new(b"abc".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();
    let count =
        <dyn Read as ReadExt>::copy_to_end_limited(reader, &mut output, 3)
            .expect("UFCS copy_to_end_limited should work on dyn Read");
    assert_eq!(3, count);
    assert_eq!(b"abc", output.as_slice());
}

#[test]
fn test_read_to_end_limited_delegates_to_impl_via_trait() {
    let mut reader = Cursor::new(b"abc".to_vec());

    let data = reader
        .read_to_end_limited(3)
        .expect("ReadExt should delegate to read_to_end_limited impl");

    assert_eq!(b"abc", data.as_slice());
}

#[test]
fn test_read_to_string_limited_delegates_to_impl_via_trait() {
    let mut reader = Cursor::new("hello".as_bytes().to_vec());

    let value = reader
        .read_to_string_limited(8)
        .expect("ReadExt should delegate to read_to_string_limited impl");

    assert_eq!("hello", value);
}

#[test]
fn test_discard_exact_or_eof_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let count = reader
        .discard_exact_or_eof(4)
        .expect("discard extension should work on dyn Read");

    assert_eq!(4, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"ef", remaining.as_slice());
}

#[test]
fn test_copy_to_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to(&mut output)
        .expect("copy extension should work on dyn Read");

    assert_eq!(6, count);
    assert_eq!(b"abcdef", output.as_slice());
}

#[test]
fn test_copy_to_at_most_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to_at_most(&mut output, 4)
        .expect("limited copy extension should work on dyn Read");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_copy_to_end_limited_works_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcd".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = Vec::new();

    let count = reader
        .copy_to_end_limited(&mut output, 4)
        .expect("end-limited copy extension should work on dyn Read");

    assert_eq!(4, count);
    assert_eq!(b"abcd", output.as_slice());
}

#[test]
fn test_discard_exact_or_eof_discards_across_short_reads() {
    let mut reader = ShortReader::new(b"abcdef", 2);

    let count = reader
        .discard_exact_or_eof(5)
        .expect("short reads should be retried while discarding");

    assert_eq!(5, count);
    let mut remaining = Vec::new();
    reader
        .read_to_end(&mut remaining)
        .expect("remaining bytes should still be readable");
    assert_eq!(b"f", remaining.as_slice());
}

#[test]
fn test_discard_exact_or_eof_returns_partial_count_at_eof() {
    let mut reader = ShortReader::new(b"abc", 1);

    let count = reader
        .discard_exact_or_eof(5)
        .expect("EOF after partial discard should not be an error");

    assert_eq!(3, count);
}

#[test]
fn test_discard_exact_or_eof_retries_interrupted_reads() {
    let mut reader = InterruptedOnceReader::new(b"abc");

    let count = reader
        .discard_exact_or_eof(3)
        .expect("interrupted reads should be retried while discarding");

    assert_eq!(3, count);
}

#[test]
fn test_discard_exact_or_eof_zero_bytes_does_not_read() {
    let mut reader = PanicOnRead;

    let count = reader
        .discard_exact_or_eof(0)
        .expect("zero-byte discard should complete immediately");

    assert_eq!(0, count);
}

#[test]
fn test_discard_exact_or_eof_returns_non_interrupted_error() {
    let mut reader = FailingReader;

    let error = reader.discard_exact_or_eof(3).expect_err(
        "non-interrupted read errors should be returned while discarding",
    );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_exact_vec_limited_into_delegates_via_trait() {
    let mut reader = Cursor::new(b"abcdef".to_vec());
    let mut output = b"prefix".to_vec();

    reader
        .read_exact_vec_limited_into(&mut output, 4, 8)
        .expect("ReadExt should delegate read_exact_vec_limited_into");

    assert_eq!(b"prefixabcd", output.as_slice());
}

#[test]
fn test_read_to_end_limited_into_delegates_via_trait() {
    let mut reader = Cursor::new(b"abc".to_vec());
    let mut output = b"seed".to_vec();

    let count = reader
        .read_to_end_limited_into(&mut output, 3)
        .expect("ReadExt should delegate read_to_end_limited_into");

    assert_eq!(3, count);
    assert_eq!(b"seedabc", output.as_slice());
}

#[test]
fn test_read_to_string_limited_into_delegates_via_trait() {
    let mut reader = Cursor::new(b"hello".to_vec());
    let mut output = String::from("pre-");

    let count = reader
        .read_to_string_limited_into(&mut output, 8)
        .expect("ReadExt should delegate read_to_string_limited_into");

    assert_eq!(5, count);
    assert_eq!("pre-hello", output);
}

#[test]
fn test_read_to_string_limited_into_rejects_invalid_utf8() {
    let mut reader = Cursor::new(vec![0xff, 0xfe]);
    let mut output = String::new();

    let error = reader
        .read_to_string_limited_into(&mut output, 4)
        .expect_err("invalid UTF-8 should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(output.is_empty());
}

#[test]
fn test_read_ext_blanket_impl_on_concrete_reader() {
    let mut reader = Cursor::new(b"abcdef".to_vec());
    let mut buffer = [b'-'; 6];

    // SAFETY: `buffer[1..4]` is a valid destination range.
    let count = unsafe {
        reader
            .read_unchecked(&mut buffer, 1, 3)
            .expect("blanket read_unchecked should work on concrete readers")
    };
    assert_eq!(3, count);
    assert_eq!(b"-abc--", &buffer);

    let array = reader
        .read_exact_array::<2>()
        .expect("blanket read_exact_array should work on concrete readers");
    assert_eq!(*b"de", array);
}

#[test]
fn test_read_to_string_limited_rejects_invalid_utf8() {
    let mut reader = Cursor::new(vec![0xff, 0xfe]);

    let error = reader
        .read_to_string_limited(4)
        .expect_err("invalid UTF-8 should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

#[test]
fn test_read_ext_blanket_impl_helpers_via_ufcs() {
    let mut reader = Cursor::new(b"abcdef".to_vec());

    let data = ReadExt::read_exact_vec_limited(&mut reader, 4, 8)
        .expect("UFCS read_exact_vec_limited should use blanket impl");
    assert_eq!(b"abcd", data.as_slice());

    let mut reader = Cursor::new(b"hello".to_vec());
    let text = ReadExt::read_to_string_limited(&mut reader, 8)
        .expect("UFCS read_to_string_limited should use blanket impl");
    assert_eq!("hello", text);

    let mut reader = Cursor::new(b"hello".to_vec());
    let mut output = String::from("x");
    let count =
        ReadExt::read_to_string_limited_into(&mut reader, &mut output, 8)
            .expect("UFCS read_to_string_limited_into should use blanket impl");
    assert_eq!(5, count);
    assert_eq!("xhello", output);
}

#[test]
fn test_read_exact_vec_limited_propagates_read_error_via_blanket_impl() {
    let mut reader = FailingReader;

    let error = reader.read_exact_vec_limited(3, 3).expect_err(
        "blanket read_exact_vec_limited should propagate read errors",
    );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_string_limited_propagates_read_error_via_blanket_impl() {
    let mut reader = FailingReader;

    let error = reader.read_to_string_limited(8).expect_err(
        "blanket read_to_string_limited should propagate read errors",
    );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_to_string_limited_into_propagates_read_error_via_blanket_impl() {
    let mut reader = FailingReader;
    let mut output = String::from("seed-");

    let error = reader
        .read_to_string_limited_into(&mut output, 8)
        .expect_err(
            "blanket read_to_string_limited_into should propagate read errors",
        );

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
    assert_eq!("seed-", output);
}

#[test]
fn test_read_ext_blanket_impl_on_dyn_read() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn Read = &mut cursor;

    let data = ReadExt::read_exact_vec_limited(reader, 4, 8)
        .expect("read_exact_vec_limited should work on dyn Read");
    assert_eq!(b"abcd", data.as_slice());

    let mut cursor = Cursor::new(b"hello".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let text = ReadExt::read_to_string_limited(reader, 8)
        .expect("read_to_string_limited should work on dyn Read");
    assert_eq!("hello", text);

    let mut cursor = Cursor::new(b"hello".to_vec());
    let reader: &mut dyn Read = &mut cursor;
    let mut output = String::from("x");
    let count = ReadExt::read_to_string_limited_into(reader, &mut output, 8)
        .expect("read_to_string_limited_into should work on dyn Read");
    assert_eq!(5, count);
    assert_eq!("xhello", output);
}
