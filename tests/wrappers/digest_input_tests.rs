// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for digesting byte input.

use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;
use std::io::SeekFrom;

use qubit_io::DigestInput;
use qubit_io::Input;
use qubit_io::Seekable;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;

use super::support_tests::ScriptedInput;

struct InterruptedSource {
    step: usize,
}

impl Read for InterruptedSource {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.step += 1;
        match self.step {
            1 => {
                output[..2].copy_from_slice(b"ab");
                Ok(2)
            }
            2 | 3 => Err(ErrorKind::Interrupted.into()),
            4 => {
                output[..2].copy_from_slice(b"cd");
                Ok(2)
            }
            _ => Ok(0),
        }
    }
}

struct OverreportedRead;

impl Input for OverreportedRead {
    type Item = u8;

    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        Ok(output.len() + 1)
    }

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count + 1)
    }
}

struct LargeInterruptedSource {
    position: usize,
    interrupted: bool,
}

impl Read for LargeInterruptedSource {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let total = qubit_io::DEFAULT_COPY_BUFFER_SIZE * 2 + 3;
        if self.position == qubit_io::DEFAULT_COPY_BUFFER_SIZE * 2 && !self.interrupted {
            self.interrupted = true;
            return Err(ErrorKind::Interrupted.into());
        }
        let count = output.len().min(total - self.position);
        output[..count].fill(b'x');
        self.position += count;
        Ok(count)
    }
}

struct InterruptedFirstSource {
    step: usize,
}

impl Read for InterruptedFirstSource {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        self.step += 1;
        match self.step {
            1 => Err(ErrorKind::Interrupted.into()),
            2 => {
                output[0] = b'a';
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

struct ProgressThenError {
    position: usize,
}

impl Read for ProgressThenError {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.position == 0 {
            output[..2].copy_from_slice(b"ab");
            self.position = 2;
            Ok(2)
        } else {
            Err(std::io::Error::other("read failure after progress"))
        }
    }
}

#[test]
fn test_digest_input_snapshots_preserve_state_and_hash_only_read_bytes() {
    let mut input = DigestInput::new(Cursor::new(b"abc"), Sha256::new());
    assert_eq!(Sha256::digest(b""), input.digest());
    let mut bytes = [0; 2];
    assert_eq!(2, input.read(&mut bytes).expect("read prefix"));
    assert_eq!(b"ab", &bytes);
    assert_eq!(Sha256::digest(b"ab"), input.digest());
    assert_eq!(input.digest(), input.digest());
    assert_eq!(1, input.read(&mut bytes).expect("short read"));
    assert_eq!(0, input.read(&mut bytes).expect("read EOF"));
    let (inner, fingerprint) = input.finalize();
    assert_eq!(3, inner.position());
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
    assert_eq!(b"\xba\x78\x16\xbf\x8f\x01\xcf\xea\x41\x41\x40\xde\x5d\xae\x22\x23\xb0\x03\x61\xa3\x96\x17\x7a\x9c\xb4\x10\xff\x61\xf2\x00\x15\xad", fingerprint.as_slice());
}

#[test]
fn test_digest_input_chunking_and_algorithm_output_sizes() {
    for chunk in [1, 2, 3, 8] {
        let mut input = DigestInput::new(Cursor::new(b"abc"), Sha512::new());
        let mut bytes = vec![0; chunk];
        while input.read(&mut bytes).expect("read chunk") != 0 {}
        let (_, fingerprint) = input.finalize();
        assert_eq!(64, fingerprint.len());
        assert_eq!(Sha512::digest(b"abc"), fingerprint);
    }
    let input = DigestInput::new(Cursor::new(b"unread"), Sha256::new());
    let (inner, fingerprint) = input.finalize();
    assert_eq!(0, inner.position());
    assert_eq!(Sha256::digest(b""), fingerprint);
}

#[test]
fn test_digest_input_prefix_accessors_seek_and_unfinished_parts() {
    let mut hasher = Sha256::new();
    hasher.update(b"prefix:");
    let mut inner = ScriptedInput::items(b"abc".to_vec());
    inner.buffered = true;
    let mut input = DigestInput::new(inner, hasher);
    assert!(input.is_buffered());
    input.inner_mut().buffered = false;
    assert!(!input.is_buffered());
    input.hasher_mut().update(b"x");
    let mut byte = [0];
    input.inner_mut().read(&mut byte).expect("bypass digest");
    input.read(&mut byte).expect("tracked byte");
    assert_eq!(Sha256::digest(b"prefix:xb"), input.hasher().clone().finalize());
    assert_eq!(7, input.seek_to(SeekFrom::Start(7)).expect("seek"));
    input.inner_mut().seek_error = Some("seek failure");
    assert_eq!(
        ErrorKind::Other,
        input.seek_to(SeekFrom::Start(0)).expect_err("seek error").kind()
    );
    let (inner, mut hasher) = input.into_parts();
    assert_eq!(7, inner.position);
    hasher.update(b"tail");
    assert_eq!(Sha256::digest(b"prefix:xbtail"), hasher.finalize());
}

#[test]
fn test_digest_input_errors_and_invalid_counts_do_not_change_digest() {
    let mut bytes = [0; 2];
    let mut input = DigestInput::new(ScriptedInput::<u8>::failing("read failure"), Sha256::new());
    assert_eq!(ErrorKind::Other, input.read(&mut bytes).expect_err("read error").kind());
    assert_eq!(Sha256::digest(b""), input.digest());
    let mut input = DigestInput::new(ScriptedInput::<u8>::invalid_count(), Sha256::new());
    assert_eq!(
        ErrorKind::InvalidData,
        input.read(&mut bytes).expect_err("invalid count").kind()
    );
    assert_eq!(Sha256::digest(b""), input.digest());
}

#[test]
fn test_digest_input_seek_tracks_transfer_history_and_indexed_reads() {
    let mut input = DigestInput::new(Cursor::new(b"abc"), Sha256::new());
    let mut bytes = [b'x'; 4];
    // SAFETY: The destination range 1..3 is within the array.
    assert_eq!(
        2,
        unsafe { input.read_unchecked(&mut bytes, 1, 2) }.expect("indexed read")
    );
    assert_eq!(b"xabx", &bytes);
    input.seek_to(SeekFrom::Start(0)).expect("rewind");
    input.read(&mut bytes[..1]).expect("repeat byte");
    assert_eq!(Sha256::digest(b"aba"), input.digest());
}

#[test]
fn test_digest_input_non_clone_algorithm_can_transfer_and_finalize() {
    let stream = std::io::Cursor::new(b"abc");
    let mut wrapper = DigestInput::new(stream, super::digest_support_tests::NonCloneDigest::default());
    let mut bytes = [0; 3];
    assert_eq!(3, wrapper.read(&mut bytes).expect("transfer bytes"));
    let (_, fingerprint) = wrapper.finalize();
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
}

#[test]
fn test_digest_input_digest_all_reads_remaining_input_with_fresh_state() {
    for buffered in [false, true] {
        let mut input = ScriptedInput::items(vec![b'x'; qubit_io::DEFAULT_COPY_BUFFER_SIZE * 2 + 3]);
        input.buffered = buffered;
        let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input).expect("digest to EOF");
        assert_eq!(
            Sha256::digest(vec![b'x'; qubit_io::DEFAULT_COPY_BUFFER_SIZE * 2 + 3]),
            fingerprint
        );
        assert_eq!(0, input.remaining_len());
    }
    let mut input = Cursor::new(b"prefix:abc");
    input.set_position(7);
    let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input).expect("digest suffix");
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
    assert_eq!(10, input.position());
    assert_eq!(
        Sha256::digest(b""),
        DigestInput::<_, Sha256>::digest_all(&mut input).expect("empty remainder")
    );
}

#[test]
fn test_digest_input_digest_all_returns_errors_and_retains_input_access() {
    let mut input = ScriptedInput::<u8>::failing("digest read failure");
    assert_eq!(
        ErrorKind::Other,
        DigestInput::<_, Sha256>::digest_all(&mut input)
            .expect_err("read failure")
            .kind()
    );
    input.action = super::support_tests::InputAction::Items(b"abc".to_vec());
    assert_eq!(
        Sha256::digest(b"abc"),
        DigestInput::<_, Sha256>::digest_all(&mut input).expect("retry input")
    );
    let mut input = ScriptedInput::<u8>::invalid_count();
    assert_eq!(
        ErrorKind::InvalidData,
        DigestInput::<_, Sha256>::digest_all(&mut input)
            .expect_err("invalid count")
            .kind()
    );
}

#[test]
fn test_digest_all_retries_interrupted_after_progress() {
    let mut input = InterruptedSource { step: 0 };
    let actual = DigestInput::<_, Sha256>::digest_all(&mut input)
        .expect("temporary interruptions must preserve the accumulated digest");
    assert_eq!(actual, Sha256::digest(b"abcd"));
}

#[test]
fn test_digest_all_rejects_overreported_read_count() {
    assert_eq!(
        ErrorKind::InvalidData,
        DigestInput::<_, Sha256>::digest_all(&mut OverreportedRead)
            .expect_err("overreported read counts are invalid data")
            .kind()
    );
}

#[test]
fn test_digest_all_returns_empty_input_digest() {
    let mut input = Cursor::new(b"");
    assert_eq!(
        Sha256::digest(b""),
        DigestInput::<_, Sha256>::digest_all(&mut input).expect("digest empty input")
    );
}

#[test]
fn test_digest_all_retries_interrupted_before_progress() {
    let mut input = InterruptedFirstSource { step: 0 };
    assert_eq!(
        Sha256::digest(b"a"),
        DigestInput::<_, Sha256>::digest_all(&mut input).expect("retry initial interruption")
    );
}

#[test]
fn test_digest_all_retries_interrupted_after_multiple_buffers() {
    let mut input = LargeInterruptedSource {
        position: 0,
        interrupted: false,
    };
    assert_eq!(
        Sha256::digest(vec![b'x'; qubit_io::DEFAULT_COPY_BUFFER_SIZE * 2 + 3]),
        DigestInput::<_, Sha256>::digest_all(&mut input).expect("retry after multiple buffers")
    );
}

#[test]
fn test_digest_all_does_not_rewind_after_non_interrupted_error() {
    let mut input = ProgressThenError { position: 0 };
    assert_eq!(
        ErrorKind::Other,
        DigestInput::<_, Sha256>::digest_all(&mut input)
            .expect_err("ordinary errors are returned")
            .kind()
    );
    assert_eq!(2, input.position);
}

#[test]
fn test_digest_input_rejects_safe_overreported_count() {
    let mut input = DigestInput::new(super::support_tests::SafeOverreportInput, Sha256::new());
    let before = input.digest();
    let mut output = [0_u8; 4];
    let error =
        unsafe { qubit_io::Input::read_unchecked(&mut input, &mut output, 1, 2) }.expect_err("overreport should fail");

    assert_eq!(ErrorKind::InvalidData, error.kind());
    assert_eq!(before, input.digest());
}
