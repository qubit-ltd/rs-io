// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::io::ErrorKind;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::ReadExactFuture;

use super::support_tests::BudgetWakeCounter;
use super::support_tests::CancellationInput;
use super::support_tests::PollResult;
use super::support_tests::SafeErrorInput;
use super::support_tests::SafeOverreportedInput;
use super::support_tests::SafeScriptedInput;
use super::support_tests::ScriptedInput;
use super::support_tests::TestInput;
use super::support_tests::assert_normalized_error;

#[test]
fn test_read_exact_future_type_is_public() {
    assert!(std::any::type_name::<ReadExactFuture<'static, TestInput>>().contains("ReadExactFuture"));
}

#[test]
fn test_read_exact_future_normalizes_safe_override_would_block() {
    let mut input = SafeErrorInput(std::io::Error::from(ErrorKind::WouldBlock));
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => assert_eq!(ErrorKind::InvalidData, error.kind()),
        other => panic!("expected normalized error, got {other:?}"),
    }
}

#[test]
fn test_read_exact_future_preserves_progress_and_normalizes_all_errors() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = SafeScriptedInput::new([
            PollResult::Read(1),
            PollResult::Error(std::io::Error::new(kind, "read-exact marker")),
        ]);
        let mut output = [0_u8; 2];
        let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
        let mut cx = Context::from_waker(Waker::noop());
        match Future::poll(Pin::new(&mut future), &mut cx) {
            Poll::Ready(Err(error)) => assert_normalized_error(&error, kind, "read-exact marker"),
            other => panic!("expected normalized error, got {other:?}"),
        }
        assert_eq!(1, future.items_read());
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = Future::poll(Pin::new(&mut future), &mut cx);
            }))
            .is_err()
        );
        drop(future);
        assert_eq!([0xA5, 0], output);
    }

    let mut input = SafeScriptedInput::new([PollResult::Error(std::io::Error::new(
        ErrorKind::PermissionDenied,
        "read-exact payload",
    ))]);
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::PermissionDenied, error.kind());
            assert_eq!("read-exact payload", error.to_string());
        }
        other => panic!("expected original error, got {other:?}"),
    }
}

#[test]
fn test_read_exact_rejects_safe_overreported_count_and_preserves_valid_progress() {
    let mut input = SafeOverreportedInput::new(1);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => assert_eq!(ErrorKind::InvalidData, error.kind()),
        other => panic!("expected InvalidData, got {other:?}"),
    }
    assert_eq!(1, future.items_read());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = Future::poll(Pin::new(&mut future), &mut cx);
        }))
        .is_err()
    );
}

#[test]
fn test_read_exact_reports_no_progress_before_initial_overreport() {
    let mut input = SafeOverreportedInput::new(0);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => assert_eq!(ErrorKind::InvalidData, error.kind()),
        other => panic!("expected InvalidData, got {other:?}"),
    }
    assert_eq!(0, future.items_read());
}

#[test]
fn test_read_exact_future_completes_immediately_for_empty_destination() {
    let mut input = TestInput;
    let mut output: [u8; 0] = [];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Ok(())) => {}
        Poll::Pending => {
            panic!("empty destination should never return pending")
        }
        Poll::Ready(Err(error)) => {
            panic!("empty read_exact should not error, got: {error}")
        }
    }
}

#[test]
fn test_read_exact_future_constructs_and_reports_eof() {
    let mut input = TestInput;
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_read());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(std::io::ErrorKind::UnexpectedEof, error.kind());
        }
        Poll::Ready(Ok(())) => panic!("exact read should report EOF"),
        Poll::Pending => panic!("test input should not pend"),
    }
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = Future::poll(Pin::new(&mut future), &mut cx);
        }))
        .is_err()
    );
}

#[test]
fn test_read_exact_future_supports_pending_reads_before_progress() {
    let mut input = ScriptedInput::new([PollResult::Pending, PollResult::Read(1), PollResult::Read(1)]);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Pending));
    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn test_read_exact_future_constructs_and_propagates_io_error() {
    let mut input = ScriptedInput::new([PollResult::Error(std::io::Error::new(
        ErrorKind::PermissionDenied,
        "denied",
    ))]);
    let mut output = [0_u8; 1];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::PermissionDenied, error.kind());
        }
        Poll::Ready(Ok(())) => {
            panic!("exact read should not swallow I/O errors")
        }
        Poll::Pending => panic!("scripted input should be immediately ready"),
    }
}

#[test]
fn test_read_exact_future_pauses_when_ready_budget_is_exhausted() {
    let mut input = ScriptedInput::new(std::iter::repeat_with(|| PollResult::Read(1)).take(65));
    let mut output = [0_u8; 65];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let counter = Arc::new(BudgetWakeCounter::default());
    let waker = Waker::from(Arc::clone(&counter));
    let mut cx = Context::from_waker(&waker);
    let before = counter.0.load(Ordering::SeqCst);

    assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Pending));
    assert_eq!(64, future.items_read());
    assert!(
        counter.0.load(Ordering::SeqCst) > before,
        "budget Pending must schedule another poll"
    );
    assert!(matches!(
        Future::poll(Pin::new(&mut future), &mut cx),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(65, future.items_read());
    drop(future);
    assert_eq!([0xA5; 65], output);
}

#[test]
fn test_read_exact_future_reports_eof_after_partial_progress() {
    let mut input = ScriptedInput::new([PollResult::Read(1), PollResult::Read(0)]);
    let mut output = [0_u8; 2];
    let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
    let mut cx = Context::from_waker(Waker::noop());

    assert_eq!(0, future.items_read());
    match Future::poll(Pin::new(&mut future), &mut cx) {
        Poll::Ready(Err(error)) => {
            assert_eq!(ErrorKind::UnexpectedEof, error.kind());
            assert_eq!(1, future.items_read());
        }
        Poll::Pending => panic!("partial read should not remain pending"),
        Poll::Ready(Ok(_)) => {
            panic!("partial read should fail with UnexpectedEof")
        }
    }
}

#[test]
fn test_read_exact_cancellation_resume() {
    for pending_after in [Some(2), None] {
        let expected: Vec<u8> = (0_u8..65).collect();
        let mut input = CancellationInput {
            bytes: expected.clone(),
            position: 0,
            pending_after,
        };
        let mut output = [255; 65];
        let mut cx = Context::from_waker(Waker::noop());
        let mut future = ReadExactFuture::new(Pin::new(&mut input), &mut output);
        assert!(matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Pending));
        let done = future.items_read();
        assert_eq!(pending_after.unwrap_or(64), done);
        drop(future);
        assert_eq!(done, input.position);
        assert_eq!(&expected[..done], &output[..done]);
        assert!(output[done..].iter().all(|byte| *byte == 255));
        let mut resumed = ReadExactFuture::new(Pin::new(&mut input), &mut output[done..]);
        assert!(matches!(
            Future::poll(Pin::new(&mut resumed), &mut cx),
            Poll::Ready(Ok(()))
        ));
        assert_eq!(65, done + resumed.items_read());
        drop(resumed);
        assert_eq!(65, input.position);
        assert_eq!(expected.as_slice(), &output);
    }
}
