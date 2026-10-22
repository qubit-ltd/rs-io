// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for asynchronous digesting byte output.

use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Waker;

use qubit_io::AsyncClose;
use qubit_io::AsyncDigestOutput;
use qubit_io::AsyncOutput;
use qubit_io::BoxAsyncOutput;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;

use super::digest_support_tests::NonCloneDigest;
use super::digest_support_tests::PinnedStream;
use super::digest_support_tests::Step;
use super::digest_support_tests::error_kind;
use super::digest_support_tests::ready;

#[test]
fn test_async_digest_output_pending_short_writes_errors_and_pinning() {
    let stream = PinnedStream::default();
    stream.state.borrow_mut().buffered = true;
    stream.state.borrow_mut().steps.extend([
        Step::Pending,
        Step::Transfer(2),
        Step::Pending,
        Step::Error(ErrorKind::Other),
        Step::InvalidCount,
        Step::Error(ErrorKind::WouldBlock),
        Step::Error(ErrorKind::Interrupted),
        Step::Transfer(1),
        Step::Transfer(0),
    ]);
    let mut output = Box::pin(AsyncDigestOutput::new(stream, Sha256::new()));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(output.is_buffered());
    assert!(output.as_mut().poll_write(&mut cx, b"abc").is_pending());
    assert_eq!(Sha256::digest(b""), output.digest());
    // SAFETY: The source range 1..3 is within the slice.
    assert_eq!(
        2,
        ready(unsafe { output.as_mut().poll_write_unchecked(&mut cx, b"xabx", 1, 2) })
    );
    assert_eq!(Sha256::digest(b"ab"), output.digest());
    assert_eq!(output.digest(), output.digest());
    assert!(output.as_mut().poll_write(&mut cx, b"c").is_pending());
    for expected in [
        ErrorKind::Other,
        ErrorKind::InvalidData,
        ErrorKind::InvalidData,
        ErrorKind::InvalidData,
    ] {
        assert_eq!(expected, error_kind(output.as_mut().poll_write(&mut cx, b"c")));
        assert_eq!(Sha256::digest(b"ab"), output.digest());
    }
    assert_eq!(1, ready(output.as_mut().poll_write(&mut cx, b"c")));
    assert_eq!(0, ready(output.as_mut().poll_write(&mut cx, b"unused")));
    assert_eq!(0, ready(output.as_mut().poll_write(&mut cx, b"")));
    assert_eq!(Sha256::digest(b"abc"), output.digest());
    assert_eq!(b"abc", output.inner().state.borrow().bytes.as_slice());
}

#[test]
fn test_async_digest_output_flush_and_close_pending_errors_and_success() {
    let stream = PinnedStream::default();
    stream.state.borrow_mut().flush_steps.extend([
        Step::Pending,
        Step::Error(ErrorKind::Other),
        Step::Error(ErrorKind::WouldBlock),
        Step::Error(ErrorKind::Interrupted),
    ]);
    stream.state.borrow_mut().close_steps.extend([
        Step::Pending,
        Step::Error(ErrorKind::Other),
        Step::Error(ErrorKind::WouldBlock),
        Step::Error(ErrorKind::Interrupted),
    ]);
    let mut output = Box::pin(AsyncDigestOutput::new(stream, Sha256::new()));
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(3, ready(output.as_mut().poll_write(&mut cx, b"abc")));
    assert!(output.as_mut().poll_flush(&mut cx).is_pending());
    for expected in [ErrorKind::Other, ErrorKind::InvalidData, ErrorKind::InvalidData] {
        assert_eq!(expected, error_kind(output.as_mut().poll_flush(&mut cx)));
    }
    ready(output.as_mut().poll_flush(&mut cx));
    assert!(output.as_mut().poll_close(&mut cx).is_pending());
    for expected in [ErrorKind::Other, ErrorKind::InvalidData, ErrorKind::InvalidData] {
        assert_eq!(expected, error_kind(output.as_mut().poll_close(&mut cx)));
    }
    ready(output.as_mut().poll_close(&mut cx));
    assert_eq!(Sha256::digest(b"abc"), output.digest());
    assert_eq!(5, output.inner().state.borrow().flush_calls);
    assert_eq!(5, output.inner().state.borrow().close_calls);
}

#[test]
fn test_async_digest_output_accessors_parts_and_finalize_perform_no_io() {
    let mut hasher = Sha512::new();
    hasher.update(b"prefix:");
    let mut output = AsyncDigestOutput::new(PinnedStream::default(), hasher);
    assert!(!output.is_buffered());
    output.inner_mut().state.get_mut().buffered = true;
    assert!(output.is_buffered());
    output.hasher_mut().update(b"tail");
    assert_eq!(Sha512::digest(b"prefix:tail"), output.hasher().clone().finalize());
    let (inner, fingerprint) = output.finalize();
    assert_eq!(64, fingerprint.len());
    assert_eq!(Sha512::digest(b"prefix:tail"), fingerprint);
    assert_eq!(0, inner.state.borrow().flush_calls);
    assert_eq!(0, inner.state.borrow().close_calls);
    let output = AsyncDigestOutput::new(inner, Sha256::new());
    let (inner, mut hasher) = output.into_parts();
    hasher.update(b"continued");
    assert_eq!(Sha256::digest(b"continued"), hasher.finalize());
    assert_eq!(0, inner.state.borrow().flush_calls);
    assert_eq!(0, inner.state.borrow().close_calls);
}

#[test]
fn test_async_digest_output_non_clone_algorithm_can_write_and_finalize() {
    let stream = BoxAsyncOutput::new(Box::new(PinnedStream::default()));
    let mut output = AsyncDigestOutput::new(stream, NonCloneDigest::default());
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(3, ready(Pin::new(&mut output).poll_write(&mut cx, b"abc")));
    let (_, fingerprint) = output.finalize();
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
}
