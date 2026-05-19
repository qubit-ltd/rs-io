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
    Cursor,
    Error,
    ErrorKind,
    Read,
    Write,
};

use qubit_io::{
    StringReadExt,
    StringWriteExt,
};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FailAfterBytesWriter {
    remaining: usize,
}

impl Write for FailAfterBytesWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(Error::other("payload failed"));
        }
        let count = buffer.len().min(self.remaining);
        self.remaining -= count;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_length_prefixed_utf8_strings_round_trip_uleb() {
    let mut buffer = Vec::new();
    buffer
        .write_utf8_string_uleb("hello 世界")
        .expect("string should be written");

    let mut input = Cursor::new(buffer);
    let value = input
        .read_utf8_string_uleb(64)
        .expect("string should be read");

    assert_eq!("hello 世界", value);
}

#[test]
fn test_read_utf8_string_uleb_strict_accepts_canonical_length() {
    let mut input = Cursor::new(vec![5, b'h', b'e', b'l', b'l', b'o']);

    let value = input
        .read_utf8_string_uleb_strict(8)
        .expect("canonical ULEB-prefixed string should be read");

    assert_eq!("hello", value);
}

#[test]
fn test_read_utf8_string_uleb_strict_rejects_noncanonical_length_before_payload() {
    let mut input = Cursor::new(vec![0x80, 0x00, b'a']);

    let error = input
        .read_utf8_string_uleb_strict(8)
        .expect_err("non-canonical ULEB length should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(2, input.position());
}

#[test]
fn test_read_utf8_string_uleb_strict_rejects_length_beyond_limit_before_payload() {
    let mut input = Cursor::new(vec![3, b'a', b'b', b'c']);

    let error = input
        .read_utf8_string_uleb_strict(2)
        .expect_err("oversized strict ULEB string should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "string length 3 exceeds maximum length of 2 bytes",
        error.to_string()
    );
    assert_eq!(1, input.position());
}

#[test]
fn test_length_prefixed_utf8_strings_round_trip_u32_be_and_le() {
    let mut buffer = Vec::new();
    buffer
        .write_utf8_string_u32_be("big")
        .expect("big-endian string should be written");
    buffer
        .write_utf8_string_u32_le("little")
        .expect("little-endian string should be written");

    let mut input = Cursor::new(buffer);

    assert_eq!("big", input.read_utf8_string_u32_be(16).unwrap());
    assert_eq!("little", input.read_utf8_string_u32_le(16).unwrap());
}

#[test]
fn test_length_prefixed_utf8_strings_round_trip_u16_be_and_le() {
    let mut buffer = Vec::new();
    buffer
        .write_utf8_string_u16_be("big")
        .expect("big-endian u16 string should be written");
    buffer
        .write_utf8_string_u16_le("little")
        .expect("little-endian u16 string should be written");

    let mut input = Cursor::new(buffer);

    assert_eq!(
        "big",
        input
            .read_utf8_string_u16_be(16)
            .expect("big-endian u16 string should be read")
    );
    assert_eq!(
        "little",
        input
            .read_utf8_string_u16_le(16)
            .expect("little-endian u16 string should be read")
    );
}

#[test]
fn test_string_write_ext_ufcs_methods_work_on_dyn_write() {
    let mut buffer = Vec::new();
    {
        let writer: &mut dyn Write = &mut buffer;
        <dyn Write as StringWriteExt>::write_utf8_string_uleb(writer, "uleb")
            .expect("UFCS uleb string write should work on dyn Write");
        <dyn Write as StringWriteExt>::write_utf8_string_u16_be(writer, "u16be")
            .expect("UFCS u16be string write should work on dyn Write");
        <dyn Write as StringWriteExt>::write_utf8_string_u16_le(writer, "u16le")
            .expect("UFCS u16le string write should work on dyn Write");
        <dyn Write as StringWriteExt>::write_utf8_string_u32_be(writer, "u32be")
            .expect("UFCS u32be string write should work on dyn Write");
        <dyn Write as StringWriteExt>::write_utf8_string_u32_le(writer, "u32le")
            .expect("UFCS u32le string write should work on dyn Write");
    }

    let mut input = Cursor::new(buffer);
    assert_eq!("uleb", input.read_utf8_string_uleb(16).unwrap());
    assert_eq!("u16be", input.read_utf8_string_u16_be(16).unwrap());
    assert_eq!("u16le", input.read_utf8_string_u16_le(16).unwrap());
    assert_eq!("u32be", input.read_utf8_string_u32_be(16).unwrap());
    assert_eq!("u32le", input.read_utf8_string_u32_le(16).unwrap());
}

#[test]
fn test_read_utf8_string_accepts_empty_string_with_zero_limit() {
    let mut input = Cursor::new(vec![0]);

    let value = input
        .read_utf8_string_uleb(0)
        .expect("empty string should be accepted");

    assert_eq!("", value);
}

#[test]
fn test_read_utf8_string_rejects_length_beyond_limit_before_reading_payload() {
    let mut input = Cursor::new(vec![3, b'a', b'b', b'c']);

    let error = input
        .read_utf8_string_uleb(2)
        .expect_err("oversized string should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "string length 3 exceeds maximum length of 2 bytes",
        error.to_string()
    );
    assert_eq!(1, input.position());
}

#[test]
fn test_read_utf8_string_u32_rejects_length_beyond_limit() {
    let mut input = Cursor::new(vec![0, 0, 0, 3, b'a', b'b', b'c']);

    let error = input
        .read_utf8_string_u32_be(2)
        .expect_err("oversized u32-prefixed string should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "string length 3 exceeds maximum length of 2 bytes",
        error.to_string()
    );
    assert_eq!(4, input.position());
}

#[test]
fn test_read_utf8_string_u16_rejects_length_beyond_limit() {
    let mut input = Cursor::new(vec![0, 3, b'a', b'b', b'c']);

    let error = input
        .read_utf8_string_u16_be(2)
        .expect_err("oversized u16-prefixed string should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(
        "string length 3 exceeds maximum length of 2 bytes",
        error.to_string()
    );
    assert_eq!(2, input.position());
}

#[test]
fn test_read_utf8_string_rejects_invalid_utf8() {
    let mut input = Cursor::new(vec![2, 0xFF, 0xFF]);

    let error = input
        .read_utf8_string_uleb(8)
        .expect_err("invalid UTF-8 should be rejected");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert!(
        error
            .to_string()
            .starts_with("length-prefixed string is not valid UTF-8")
    );
}

#[test]
fn test_read_utf8_string_returns_payload_read_error() {
    let mut input = Cursor::new(vec![3, b'a']);

    let error = input
        .read_utf8_string_uleb(8)
        .expect_err("short payload should be returned as EOF");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_read_utf8_string_returns_underlying_read_error() {
    let mut input = FailingReader;

    let error = input
        .read_utf8_string_uleb(8)
        .expect_err("length read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("read failed", error.to_string());
}

#[test]
fn test_read_utf8_string_u32_returns_length_read_error() {
    let mut input = FailingReader;

    let be_error = input
        .read_utf8_string_u32_be(8)
        .expect_err("big-endian length read error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("read failed", be_error.to_string());

    let le_error = input
        .read_utf8_string_u32_le(8)
        .expect_err("little-endian length read error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("read failed", le_error.to_string());
}

#[test]
fn test_read_utf8_string_u16_returns_length_read_error() {
    let mut input = FailingReader;

    let be_error = input
        .read_utf8_string_u16_be(8)
        .expect_err("big-endian u16 length read error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("read failed", be_error.to_string());

    let le_error = input
        .read_utf8_string_u16_le(8)
        .expect_err("little-endian u16 length read error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("read failed", le_error.to_string());
}

#[test]
fn test_write_utf8_string_u32_returns_length_write_error() {
    let mut output = FailingWriter;

    let be_error = output
        .write_utf8_string_u32_be("abc")
        .expect_err("big-endian length write error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("write failed", be_error.to_string());

    let le_error = output
        .write_utf8_string_u32_le("abc")
        .expect_err("little-endian length write error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("write failed", le_error.to_string());
}

#[test]
fn test_write_utf8_string_u16_returns_length_write_error() {
    let mut output = FailingWriter;

    let be_error = output
        .write_utf8_string_u16_be("abc")
        .expect_err("big-endian u16 length write error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("write failed", be_error.to_string());

    let le_error = output
        .write_utf8_string_u16_le("abc")
        .expect_err("little-endian u16 length write error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("write failed", le_error.to_string());
}

#[test]
fn test_write_utf8_string_u16_rejects_length_overflow() {
    let mut be_output = Vec::new();
    let mut le_output = Vec::new();
    let oversized = "a".repeat(u16::MAX as usize + 1);

    let be_error = be_output
        .write_utf8_string_u16_be(&oversized)
        .expect_err("big-endian u16 length overflow should be rejected");

    assert_eq!(ErrorKind::InvalidInput, be_error.kind());
    assert_eq!(
        "string length 65536 exceeds maximum encodable u16 length",
        be_error.to_string()
    );

    let le_error = le_output
        .write_utf8_string_u16_le(&oversized)
        .expect_err("little-endian u16 length overflow should be rejected");

    assert_eq!(ErrorKind::InvalidInput, le_error.kind());
    assert_eq!(
        "string length 65536 exceeds maximum encodable u16 length",
        le_error.to_string()
    );
}

#[test]
fn test_write_utf8_string_returns_underlying_write_error_after_length_prefix() {
    let mut output = FailingWriter;

    let error = output
        .write_utf8_string_uleb("abc")
        .expect_err("payload write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
}

#[test]
fn test_write_utf8_string_returns_payload_write_error_after_uleb_length_prefix() {
    let mut output = FailAfterBytesWriter { remaining: 1 };

    let error = output
        .write_utf8_string_uleb("abc")
        .expect_err("payload write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("payload failed", error.to_string());
}

#[test]
fn test_write_utf8_string_returns_payload_write_error_after_u32_length_prefix() {
    let mut be_output = FailAfterBytesWriter { remaining: 4 };
    let be_error = be_output
        .write_utf8_string_u32_be("abc")
        .expect_err("big-endian payload write error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("payload failed", be_error.to_string());

    let mut le_output = FailAfterBytesWriter { remaining: 4 };
    let le_error = le_output
        .write_utf8_string_u32_le("abc")
        .expect_err("little-endian payload write error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("payload failed", le_error.to_string());
}

#[test]
fn test_write_utf8_string_returns_payload_write_error_after_u16_length_prefix() {
    let mut be_output = FailAfterBytesWriter { remaining: 2 };
    let be_error = be_output
        .write_utf8_string_u16_be("abc")
        .expect_err("big-endian u16 payload write error should be returned");
    assert_eq!(ErrorKind::Other, be_error.kind());
    assert_eq!("payload failed", be_error.to_string());

    let mut le_output = FailAfterBytesWriter { remaining: 2 };
    let le_error = le_output
        .write_utf8_string_u16_le("abc")
        .expect_err("little-endian u16 payload write error should be returned");
    assert_eq!(ErrorKind::Other, le_error.kind());
    assert_eq!("payload failed", le_error.to_string());
}
