// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::ChecksumInput`].

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::ChecksumInput;
use qubit_io::Input;
use qubit_io::Seekable;

use super::support_tests::ScriptedInput;

fn expected_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[test]
fn test_checksum_input_hashes_successful_bytes_and_exposes_accessors() {
    let mut input = ChecksumInput::new(Cursor::new(b"abc".to_vec()), DefaultHasher::new());
    input.inner_mut().set_position(1);
    input.hasher_mut().write(b"x");
    let mut output = [0_u8; 2];

    assert!(!input.is_buffered());
    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!(b"bc", &output);
    assert_eq!(3, input.inner().position());
    assert_eq!(expected_checksum(b"xbc"), input.checksum());
    assert_eq!(expected_checksum(b"xbc"), input.hasher().finish());

    let (source, hasher) = input.into_parts();
    assert_eq!(3, source.position());
    assert_eq!(expected_checksum(b"xbc"), hasher.finish());
}

#[test]
fn test_checksum_input_does_not_hash_failed_or_invalid_reads() {
    let mut failing = ChecksumInput::new(ScriptedInput::<u8>::failing("read failed"), DefaultHasher::new());
    let mut output = [0_u8; 2];
    let error = failing.read(&mut output).expect_err("read error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(expected_checksum(b""), failing.checksum());

    let mut invalid = ChecksumInput::new(ScriptedInput::<u8>::invalid_count(), DefaultHasher::new());
    let error = invalid
        .read(&mut output)
        .expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(expected_checksum(b""), invalid.checksum());
}

#[test]
fn test_checksum_input_forwards_seek_without_hashing() {
    let mut input = ChecksumInput::new(ScriptedInput::items(vec![1_u8]), DefaultHasher::new());

    assert_eq!(6, input.seek_to(SeekFrom::Start(6)).expect("seek should succeed"));
    assert_eq!(expected_checksum(b""), input.checksum());
}
