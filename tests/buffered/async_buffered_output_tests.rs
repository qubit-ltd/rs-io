// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::cell::RefCell;
use std::io;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use qubit_io::AsyncBufferedOutput;
use qubit_io::AsyncClose;
use qubit_io::AsyncOutput;

use super::support_tests::TestOutput;

#[test]
fn test_async_buffered_output_uses_requested_capacity() {
    let output = AsyncBufferedOutput::with_capacity(TestOutput, 3);
    assert_eq!(3, output.capacity());
}

#[test]
fn test_async_buffered_output_try_with_capacity_reports_allocation_failure() {
    assert!(
        AsyncBufferedOutput::try_with_capacity(TestOutput, usize::MAX).is_err(),
        "an unallocatable capacity should return an allocation error"
    );
}

struct OverreportOutput {
    overreport: bool,
    overreport_after_first: bool,
    calls: usize,
    written: Vec<u8>,
}

impl AsyncOutput for OverreportOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        _count: usize,
    ) -> Poll<io::Result<usize>> {
        panic!("the safe poll_write override must be used")
    }

    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<io::Result<usize>> {
        self.calls += 1;
        if self.overreport && (!self.overreport_after_first || self.calls > 1) {
            Poll::Ready(Ok(input.len() + 1))
        } else {
            let written = if self.overreport_after_first {
                input.len().min(1)
            } else {
                input.len()
            };
            self.written.extend_from_slice(&input[..written]);
            Poll::Ready(Ok(written))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncClose for OverreportOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

fn assert_invalid_data(result: Poll<io::Result<impl Sized>>) {
    match result {
        Poll::Ready(Err(error)) => assert_eq!(io::ErrorKind::InvalidData, error.kind()),
        other => panic!(
            "expected InvalidData, got poll result variant: {}",
            match other {
                Poll::Pending => "Pending",
                Poll::Ready(Ok(_)) => "Ready(Ok)",
                Poll::Ready(Err(_)) => "Ready(Err)",
            }
        ),
    }
}

#[test]
fn test_async_buffered_output_async_buffered_overreport_drain_and_direct() {
    let inner = OverreportOutput {
        overreport: true,
        overreport_after_first: false,
        calls: 0,
        written: Vec::new(),
    };
    let mut wrapper = Box::pin(AsyncBufferedOutput::with_capacity(inner, 4));
    let mut cx = context();
    let accepted = unsafe { wrapper.as_mut().poll_write_unchecked(&mut cx, b"ab", 0, 2) };
    assert!(matches!(accepted, Poll::Ready(Ok(2))));
    assert_invalid_data(wrapper.as_mut().poll_flush(&mut cx));
    assert_eq!(b"ab", wrapper.pending());

    let inner = OverreportOutput {
        overreport: true,
        overreport_after_first: false,
        calls: 0,
        written: Vec::new(),
    };
    let mut wrapper = Box::pin(AsyncBufferedOutput::with_capacity(inner, 4));
    let result = unsafe { wrapper.as_mut().poll_write_unchecked(&mut cx, b"abcd", 0, 4) };
    assert_invalid_data(result);
    assert!(wrapper.pending().is_empty());
}

#[test]
fn test_async_buffered_output_async_buffered_overreport_recovery() {
    let inner = OverreportOutput {
        overreport: true,
        overreport_after_first: true,
        calls: 0,
        written: Vec::new(),
    };
    let mut wrapper = Box::pin(AsyncBufferedOutput::with_capacity(inner, 4));
    let mut cx = context();
    let accepted = unsafe { wrapper.as_mut().poll_write_unchecked(&mut cx, b"ab", 0, 2) };
    assert!(matches!(accepted, Poll::Ready(Ok(2))));
    assert_invalid_data(wrapper.as_mut().poll_flush(&mut cx));
    assert_eq!(b"b", wrapper.pending());
    wrapper.as_mut().get_mut().inner_mut().overreport = false;
    assert!(matches!(wrapper.as_mut().poll_flush(&mut cx), Poll::Ready(Ok(()))));
    assert_eq!(b"ab".as_slice(), wrapper.as_ref().get_ref().inner().written.as_slice());
    assert!(wrapper.pending().is_empty());
}

struct PinnedOutputState {
    written: Vec<u8>,
    pending_write: usize,
    pending_flush: usize,
    pending_close: usize,
    flushes: usize,
    closes: usize,
}

struct PinnedOutput {
    state: Rc<RefCell<PinnedOutputState>>,
    address: Rc<Cell<Option<usize>>>,
    polls: Rc<Cell<usize>>,
    _pinned: PhantomPinned,
}

impl PinnedOutput {
    fn observe(self: Pin<&Self>) {
        let this = self.get_ref();
        let address = this as *const Self as usize;
        if let Some(previous) = this.address.get() {
            assert_eq!(previous, address);
        } else {
            this.address.set(Some(address));
        }
        this.polls.set(this.polls.get() + 1);
    }
}

impl AsyncOutput for PinnedOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        self.as_ref().observe();
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        if state.pending_write > 0 {
            state.pending_write -= 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let written = count.min(2);
        state.written.extend_from_slice(&input[index..index + written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.as_ref().observe();
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        state.flushes += 1;
        if state.pending_flush > 0 {
            state.pending_flush -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncClose for PinnedOutput {
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.as_ref().observe();
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        state.closes += 1;
        if state.pending_close > 0 {
            state.pending_close -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

fn pinned_output() -> PinnedOutput {
    PinnedOutput {
        state: Rc::new(RefCell::new(PinnedOutputState {
            written: Vec::new(),
            pending_write: 1,
            pending_flush: 1,
            pending_close: 1,
            flushes: 0,
            closes: 0,
        })),
        address: Rc::new(Cell::new(None)),
        polls: Rc::new(Cell::new(0)),
        _pinned: PhantomPinned,
    }
}

struct FlagWake(Arc<AtomicBool>);
impl Wake for FlagWake {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn drive_output<T>(mut poll: impl FnMut(&mut Context<'_>) -> Poll<io::Result<T>>) -> T {
    let woke = Arc::new(AtomicBool::new(false));
    let waker = Waker::from(Arc::new(FlagWake(woke.clone())));
    let mut cx = Context::from_waker(&waker);
    for _ in 0..64 {
        woke.store(false, Ordering::SeqCst);
        match poll(&mut cx) {
            Poll::Ready(Ok(value)) => return value,
            Poll::Ready(Err(error)) => panic!("pinned output failed: {error}"),
            Poll::Pending => assert!(woke.load(Ordering::SeqCst), "Pending must wake"),
        }
    }
    panic!("pinned output exceeded the poll limit")
}

#[test]
fn test_async_buffered_output_async_buffered_non_unpin_buffered_write_flush_close() {
    let inner = pinned_output();
    let observer_address = inner.address.clone();
    let polls = inner.polls.clone();
    let state = inner.state.clone();
    let mut wrapper = Box::pin(AsyncBufferedOutput::with_capacity(inner, 4));
    let first = drive_output(|cx| unsafe { wrapper.as_mut().poll_write_unchecked(cx, b"ab", 0, 2) });
    assert_eq!(2, first);
    assert_eq!(b"ab", wrapper.pending());
    let second = drive_output(|cx| unsafe { wrapper.as_mut().poll_write_unchecked(cx, b"cdef", 0, 4) });
    assert_eq!(2, second);
    let third = drive_output(|cx| unsafe { wrapper.as_mut().poll_write_unchecked(cx, b"cdef", 2, 2) });
    assert_eq!(2, third);
    assert_eq!(b"ef", wrapper.pending());
    drive_output(|cx| wrapper.as_mut().poll_flush(cx));
    assert!(wrapper.pending().is_empty());
    drive_output(|cx| wrapper.as_mut().poll_close(cx));
    assert_eq!(b"abcdef".as_slice(), state.borrow().written.as_slice());
    assert_eq!(2, state.borrow().flushes);
    assert_eq!(2, state.borrow().closes);
    assert!(observer_address.get().is_some());
    assert!(polls.get() > 1);
}

#[test]
fn test_async_buffered_output_async_buffered_non_unpin_direct_write() {
    let inner = pinned_output();
    let observer_address = inner.address.clone();
    let polls = inner.polls.clone();
    let state = inner.state.clone();
    let mut wrapper = Box::pin(AsyncBufferedOutput::with_capacity(inner, 4));
    let written = drive_output(|cx| unsafe { wrapper.as_mut().poll_write_unchecked(cx, b"abcd", 0, 4) });
    assert_eq!(2, written);
    let written = drive_output(|cx| unsafe { wrapper.as_mut().poll_write_unchecked(cx, b"abcd", 2, 2) });
    assert_eq!(2, written);
    drive_output(|cx| wrapper.as_mut().poll_flush(cx));
    assert_eq!(b"abcd".as_slice(), state.borrow().written.as_slice());
    assert!(observer_address.get().is_some());
    assert!(polls.get() > 1);
}
