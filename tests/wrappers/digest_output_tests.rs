// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for digesting byte output.

use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::DigestOutput;
use qubit_io::Output;
use qubit_io::Seekable;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;

use super::support_tests::ScriptedOutput;

#[test]
fn test_digest_output_short_writes_snapshots_and_finalization() {
    let mut output = DigestOutput::new(ScriptedOutput::short(2), Sha256::new());
    assert_eq!(Sha256::digest(b""), output.digest());
    assert_eq!(2, output.write(b"abc").expect("short write"));
    assert_eq!(Sha256::digest(b"ab"), output.digest());
    assert_eq!(output.digest(), output.digest());
    output.write_fully(b"c").expect("write suffix");
    assert_eq!(0, output.write(b"").expect("empty write"));
    let (inner, fingerprint) = output.finalize();
    assert_eq!(b"abc", inner.items.as_slice());
    assert_eq!(0, inner.flush_calls);
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
    assert_eq!(b"\xba\x78\x16\xbf\x8f\x01\xcf\xea\x41\x41\x40\xde\x5d\xae\x22\x23\xb0\x03\x61\xa3\x96\x17\x7a\x9c\xb4\x10\xff\x61\xf2\x00\x15\xad", fingerprint.as_slice());
}

#[test]
fn test_digest_output_chunking_and_algorithm_output_sizes() {
    for chunk in [1, 2, 3, 8] {
        let mut output = DigestOutput::new(ScriptedOutput::short(chunk), Sha512::new());
        output.write_fully(b"abc").expect("write all chunks");
        let (_, fingerprint) = output.finalize();
        assert_eq!(64, fingerprint.len());
        assert_eq!(Sha512::digest(b"abc"), fingerprint);
    }
    let (_, fingerprint) = DigestOutput::new(Vec::<u8>::new(), Sha256::new()).finalize();
    assert_eq!(Sha256::digest(b""), fingerprint);
}

#[test]
fn test_digest_output_prefix_accessors_and_parts_do_not_flush() {
    let mut hasher = Sha256::new();
    hasher.update(b"prefix:");
    let mut output = DigestOutput::new(ScriptedOutput::accepting(), hasher);
    assert!(!output.is_buffered());
    output.inner_mut().buffered = true;
    assert!(output.is_buffered());
    output.inner_mut().write_fully(b"bypass").expect("direct write");
    output.hasher_mut().update(b"x");
    // SAFETY: The source range 1..3 is within the slice.
    assert_eq!(
        2,
        unsafe { output.write_unchecked(b"yabz", 1, 2) }.expect("indexed write")
    );
    assert_eq!(Sha256::digest(b"prefix:xab"), output.hasher().clone().finalize());
    let (inner, mut hasher) = output.into_parts();
    assert_eq!(b"bypassab", inner.items.as_slice());
    assert_eq!(0, inner.flush_calls);
    hasher.update(b"tail");
    assert_eq!(Sha256::digest(b"prefix:xabtail"), hasher.finalize());
}

#[test]
fn test_digest_output_rejects_safe_overreported_count() {
    let mut output = DigestOutput::new(super::support_tests::SafeOverreportOutput, Sha256::new());
    let before = output.digest();
    let error =
        unsafe { qubit_io::Output::write_unchecked(&mut output, b"abcd", 1, 2) }.expect_err("overreport should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(before, output.digest());
}

#[test]
fn test_digest_output_errors_invalid_counts_and_zero_progress() {
    let mut output = DigestOutput::new(ScriptedOutput::<u8>::failing_write("write failure"), Sha256::new());
    assert_eq!(ErrorKind::Other, output.write(b"abc").expect_err("write error").kind());
    assert_eq!(Sha256::digest(b""), output.digest());
    let mut output = DigestOutput::new(ScriptedOutput::<u8>::invalid_count(), Sha256::new());
    assert_eq!(
        ErrorKind::InvalidData,
        output.write(b"abc").expect_err("invalid count").kind()
    );
    assert_eq!(Sha256::digest(b""), output.digest());
    let mut output = DigestOutput::new(ScriptedOutput::short(0), Sha256::new());
    assert_eq!(0, output.write(b"abc").expect("zero progress"));
    assert_eq!(Sha256::digest(b""), output.digest());
}

#[test]
fn test_digest_output_flush_and_seek_forward_without_resetting_digest() {
    let mut output = DigestOutput::new(ScriptedOutput::accepting(), Sha256::new());
    output.write_fully(b"abc").expect("write bytes");
    output.flush().expect("flush");
    assert_eq!(1, output.inner().flush_calls);
    assert_eq!(9, output.seek_to(SeekFrom::Start(9)).expect("seek"));
    output.inner_mut().seek_error = Some("seek failure");
    assert_eq!(
        ErrorKind::Other,
        output.seek_to(SeekFrom::Start(0)).expect_err("seek error").kind()
    );
    output.inner_mut().flush_error = Some("flush failure");
    assert_eq!(ErrorKind::Other, output.flush().expect_err("flush error").kind());
    assert_eq!(Sha256::digest(b"abc"), output.digest());
    let (inner, _) = output.finalize();
    assert_eq!(2, inner.flush_calls);
}

#[test]
fn test_digest_output_non_clone_algorithm_can_transfer_and_finalize() {
    let stream = Vec::<u8>::new();
    let mut wrapper = DigestOutput::new(stream, super::digest_support_tests::NonCloneDigest::default());

    assert_eq!(3, wrapper.write(b"abc").expect("transfer bytes"));
    let (_, fingerprint) = wrapper.finalize();
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
}
