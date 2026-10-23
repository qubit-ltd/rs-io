// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for [`qubit_io::ChecksumOutput`].

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::ErrorKind;
use std::io::SeekFrom;

#[derive(Default)]
struct RecordingHasher {
    calls: Vec<Vec<u8>>,
}

impl Hasher for RecordingHasher {
    fn finish(&self) -> u64 {
        self.calls.len() as u64
    }
    fn write(&mut self, bytes: &[u8]) {
        self.calls.push(bytes.to_vec());
    }
}

use qubit_io::ChecksumOutput;
use qubit_io::Output;
use qubit_io::Seekable;

use super::support_tests::ScriptedOutput;

fn expected_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[test]
fn test_checksum_output_hashes_successful_prefix_and_exposes_accessors() {
    let mut output = ChecksumOutput::new(ScriptedOutput::short(2), DefaultHasher::new());
    output.inner_mut().items.extend_from_slice(b"x");
    output.hasher_mut().write(b"y");

    assert!(!output.is_buffered());
    assert_eq!(2, output.write(b"abc").expect("write should succeed"));
    assert_eq!(b"xab", output.inner().items.as_slice());
    assert_eq!(expected_checksum(b"yab"), output.checksum());
    assert_eq!(expected_checksum(b"yab"), output.hasher().finish());

    let (inner, hasher) = output.into_parts();
    assert_eq!(b"xab", inner.items.as_slice());
    assert_eq!(expected_checksum(b"yab"), hasher.finish());
}

#[test]
fn test_checksum_output_does_not_hash_failed_or_invalid_writes() {
    let mut failing = ChecksumOutput::new(ScriptedOutput::<u8>::failing_write("failed"), DefaultHasher::new());
    let error = failing.write(b"abc").expect_err("write error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(expected_checksum(b""), failing.checksum());

    let mut invalid = ChecksumOutput::new(ScriptedOutput::<u8>::invalid_count(), DefaultHasher::new());
    let error = invalid.write(b"abc").expect_err("invalid progress should be rejected");
    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(expected_checksum(b""), invalid.checksum());
}

#[test]
fn test_checksum_output_forwards_flush_and_seek_without_hashing() {
    let mut output = ChecksumOutput::new(ScriptedOutput::<u8>::accepting(), DefaultHasher::new());

    output.flush().expect("flush should succeed");
    assert_eq!(1, output.inner().flush_calls);
    assert_eq!(5, output.seek_to(SeekFrom::Start(5)).expect("seek should succeed"));
    assert_eq!(expected_checksum(b""), output.checksum());
}

#[test]
fn test_checksum_output_rejects_safe_overreported_count() {
    let mut wrapper = qubit_io::ChecksumOutput::new(
        super::support_tests::SafeOverreportOutput,
        std::collections::hash_map::DefaultHasher::new(),
    );
    let before = std::hash::Hasher::finish(wrapper.hasher());
    let error = unsafe { qubit_io::Output::write_unchecked(&mut wrapper, b"abcd", 1, 2) }.unwrap_err();
    assert_eq!(std::io::ErrorKind::InvalidData, error.kind());
    assert_eq!(before, std::hash::Hasher::finish(wrapper.hasher()));
}

#[test]
fn test_checksum_output_preserves_nonempty_chunks_and_skips_empty_writes() {
    let mut output = ChecksumOutput::new(ScriptedOutput::<u8>::accepting(), RecordingHasher::default());
    assert_eq!(0, output.write(b"").expect("empty write should succeed"));
    assert_eq!(1, output.write(b"a").expect("first write should succeed"));
    assert_eq!(2, output.write(b"bc").expect("second write should succeed"));
    assert_eq!(vec![b"a".to_vec(), b"bc".to_vec()], output.hasher().calls);
}
