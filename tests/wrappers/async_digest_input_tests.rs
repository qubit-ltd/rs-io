// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Contract tests for asynchronous digesting byte input.

use std::error::Error as StdError;
use std::future::Future;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use qubit_io::AsyncDigestInput;
use qubit_io::AsyncInput;
use qubit_io::BoxAsyncInput;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;

use super::digest_support_tests::NonCloneDigest;
use super::digest_support_tests::PinnedStream;
use super::digest_support_tests::Step;
use super::digest_support_tests::error_kind;
use super::digest_support_tests::ready;
use super::support_tests::SafeOverreportInput;

#[derive(Default)]
struct BudgetWakeCounter(AtomicUsize);

impl Wake for BudgetWakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct OverreportedSafeInput {
    transferred: bool,
}

struct ForbiddenErrorInput;

impl AsyncInput for ForbiddenErrorInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        panic!("the safe override must be used")
    }

    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut [u8]) -> Poll<Result<usize>> {
        Poll::Ready(Err(Error::new(ErrorKind::WouldBlock, "digest source would-block")))
    }
}

struct PrefixThenForbiddenErrorInput {
    transferred: bool,
    calls: usize,
}

impl AsyncInput for PrefixThenForbiddenErrorInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        panic!("the safe override must be used")
    }

    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<Result<usize>> {
        self.calls += 1;
        if self.transferred {
            Poll::Ready(Err(Error::new(ErrorKind::Interrupted, "digest prefix failure")))
        } else {
            self.transferred = true;
            output[0] = b'a';
            Poll::Ready(Ok(1))
        }
    }
}

impl AsyncInput for OverreportedSafeInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        panic!("the safe override must be used")
    }

    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if this.transferred {
            Poll::Ready(Ok(output.len() + 1))
        } else {
            this.transferred = true;
            output[0] = b'a';
            Poll::Ready(Ok(1))
        }
    }
}

#[test]
fn test_async_digest_input_pending_partial_reads_errors_and_pinning() {
    let stream = PinnedStream::default();
    stream.state.borrow_mut().bytes = b"abc".to_vec();
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
    let mut input = Box::pin(AsyncDigestInput::new(stream, Sha256::new()));
    let mut cx = Context::from_waker(Waker::noop());
    let mut bytes = [b'x'; 4];
    assert!(input.is_buffered());
    assert!(input.as_mut().poll_read(&mut cx, &mut bytes).is_pending());
    assert_eq!(Sha256::digest(b""), input.digest());
    // SAFETY: The destination range 1..3 is within the array.
    assert_eq!(
        2,
        ready(unsafe { input.as_mut().poll_read_unchecked(&mut cx, &mut bytes, 1, 2) })
    );
    assert_eq!(b"xabx", &bytes);
    assert_eq!(Sha256::digest(b"ab"), input.digest());
    assert_eq!(input.digest(), input.digest());
    assert!(input.as_mut().poll_read(&mut cx, &mut bytes).is_pending());
    for expected in [
        ErrorKind::Other,
        ErrorKind::InvalidData,
        ErrorKind::InvalidData,
        ErrorKind::InvalidData,
    ] {
        assert_eq!(expected, error_kind(input.as_mut().poll_read(&mut cx, &mut bytes)));
        assert_eq!(Sha256::digest(b"ab"), input.digest());
    }
    assert_eq!(1, ready(input.as_mut().poll_read(&mut cx, &mut bytes)));
    assert_eq!(0, ready(input.as_mut().poll_read(&mut cx, &mut bytes)));
    assert_eq!(0, ready(input.as_mut().poll_read(&mut cx, &mut bytes)));
    assert_eq!(Sha256::digest(b"abc"), input.digest());
    assert_eq!(3, input.inner().state.borrow().position);
}

#[test]
fn test_async_digest_input_accessors_initial_state_and_owned_finalization() {
    let mut hasher = Sha512::new();
    hasher.update(b"prefix:");
    let mut input = AsyncDigestInput::new(PinnedStream::default(), hasher);
    assert!(!input.is_buffered());
    input.inner_mut().state.get_mut().buffered = true;
    assert!(input.is_buffered());
    input.hasher_mut().update(b"tail");
    assert_eq!(Sha512::digest(b"prefix:tail"), input.hasher().clone().finalize());
    let (inner, fingerprint) = input.finalize();
    assert_eq!(64, fingerprint.len());
    assert_eq!(Sha512::digest(b"prefix:tail"), fingerprint);
    assert_eq!(0, inner.state.borrow().position);
    let input = AsyncDigestInput::new(inner, Sha256::new());
    let (_, mut hasher) = input.into_parts();
    hasher.update(b"continued");
    assert_eq!(Sha256::digest(b"continued"), hasher.finalize());
}

#[test]
fn test_async_digest_input_non_clone_algorithm_can_read_and_finalize() {
    let stream = PinnedStream::default();
    stream.state.borrow_mut().bytes = b"abc".to_vec();
    let stream = BoxAsyncInput::new(Box::new(stream));
    let mut input = AsyncDigestInput::new(stream, NonCloneDigest::default());
    let mut cx = Context::from_waker(Waker::noop());
    let mut bytes = [0; 3];
    assert_eq!(3, ready(Pin::new(&mut input).poll_read(&mut cx, &mut bytes)));
    let (_, fingerprint) = input.finalize();
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
}

#[test]
fn test_async_digest_input_digest_all_preserves_progress_across_pending() {
    let mut stream = Box::pin(PinnedStream::default());
    stream.state.borrow_mut().bytes = b"abc".to_vec();
    stream
        .state
        .borrow_mut()
        .steps
        .extend([Step::Transfer(2), Step::Pending]);
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    let fingerprint = ready(Pin::new(&mut future).poll(&mut cx));
    assert_eq!(Sha256::digest(b"abc"), fingerprint);
    drop(future);
    assert_eq!(3, stream.state.borrow().position);
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
    assert_eq!(Sha256::digest(b""), ready(Pin::new(&mut future).poll(&mut cx)));
}

#[test]
fn test_async_digest_all_normalizes_safe_override_error() {
    let mut input = ForbiddenErrorInput;
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(Pin::new(&mut input));
    let mut cx = Context::from_waker(Waker::noop());
    match Pin::new(&mut future).poll(&mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::InvalidData, error.kind());
            assert!(
                error
                    .get_ref()
                    .and_then(StdError::source)
                    .is_some_and(|source| source.to_string().contains("digest source would-block"))
            );
        }
        other => panic!("expected normalized error, got {other:?}"),
    }
}

#[test]
fn test_async_digest_all_discards_partial_digest_after_normalized_error() {
    let mut input = PrefixThenForbiddenErrorInput {
        transferred: false,
        calls: 0,
    };
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(Pin::new(&mut input));
    let mut cx = Context::from_waker(Waker::noop());
    match Pin::new(&mut future).poll(&mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::InvalidData, error.kind());
            assert!(
                error
                    .get_ref()
                    .and_then(StdError::source)
                    .is_some_and(|source| source.to_string().contains("digest prefix failure"))
            );
        }
        other => panic!("expected normalized error without a partial digest, got {other:?}"),
    }
    assert!(input.transferred);
    assert_eq!(2, input.calls);
}

#[test]
fn test_async_digest_input_digest_all_rejects_safe_overreported_count() {
    let mut input = OverreportedSafeInput { transferred: true };
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(Pin::new(&mut input));
    let mut cx = Context::from_waker(Waker::noop());
    let result = Pin::new(&mut future).poll(&mut cx);
    assert!(matches!(result, Poll::Ready(Err(error)) if error.kind() == ErrorKind::InvalidData));
}

#[test]
fn test_async_digest_input_digest_all_rejects_overreport_after_progress() {
    let mut input = OverreportedSafeInput { transferred: false };
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(Pin::new(&mut input));
    let mut cx = Context::from_waker(Waker::noop());
    let result = Pin::new(&mut future).poll(&mut cx);
    assert!(matches!(result, Poll::Ready(Err(error)) if error.kind() == ErrorKind::InvalidData));
    assert!(input.transferred);
}

#[test]
fn test_async_digest_input_digest_all_budget_cancellation_and_errors() {
    let counter = Arc::new(BudgetWakeCounter::default());
    let waker = Waker::from(Arc::clone(&counter));
    let mut cx = Context::from_waker(&waker);
    let mut stream = Box::pin(PinnedStream::default());
    stream.state.borrow_mut().bytes = vec![b'x'; 65];
    stream
        .state
        .borrow_mut()
        .steps
        .extend((0..65).map(|_| Step::Transfer(1)));
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
    assert_eq!(0, counter.0.load(Ordering::SeqCst));
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    assert!(
        counter.0.load(Ordering::SeqCst) > 0,
        "budget Pending must schedule a wake"
    );
    assert_eq!(Sha256::digest([b'x'; 65]), ready(Pin::new(&mut future).poll(&mut cx)));
    drop(future);
    for (step, expected) in [
        (Step::InvalidCount, ErrorKind::InvalidData),
        (Step::Error(ErrorKind::Other), ErrorKind::Other),
    ] {
        stream.state.borrow_mut().steps.push_back(step);
        let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
        assert_eq!(expected, error_kind(Pin::new(&mut future).poll(&mut cx)));
    }
    stream.state.borrow_mut().position = 0;
    stream
        .state
        .borrow_mut()
        .steps
        .extend([Step::Transfer(2), Step::Pending]);
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
    assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
    drop(future);
    assert_eq!(2, stream.state.borrow().position);
}

#[test]
#[should_panic(expected = "DigestAllFuture polled after completion")]
fn test_async_digest_input_digest_all_rejects_poll_after_completion() {
    let mut stream = Box::pin(PinnedStream::default());
    let mut future = AsyncDigestInput::<_, Sha256>::digest_all(stream.as_mut());
    let mut cx = Context::from_waker(Waker::noop());
    let _ = ready(Pin::new(&mut future).poll(&mut cx));
    let _ = Pin::new(&mut future).poll(&mut cx);
}

#[test]
fn test_async_digest_input_rejects_safe_overreported_count() {
    let mut wrapper = AsyncDigestInput::new(SafeOverreportInput, Sha256::new());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let before = wrapper.digest();
    let mut bytes = [0_u8; 2];
    let result = AsyncInput::poll_read(Pin::new(&mut wrapper), &mut cx, &mut bytes);
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(before, wrapper.digest());
}
