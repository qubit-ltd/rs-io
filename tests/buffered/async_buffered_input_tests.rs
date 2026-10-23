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

use qubit_io::AsyncBufferedInput;
use qubit_io::AsyncInput;

use super::support_tests::TestInput;

#[test]
fn test_async_buffered_input_uses_requested_capacity() {
    let input = AsyncBufferedInput::with_capacity(TestInput, 3);
    assert_eq!(3, input.capacity());
}

#[test]
fn test_async_buffered_input_try_with_capacity_reports_allocation_failure() {
    assert!(AsyncBufferedInput::try_with_capacity(TestInput, usize::MAX).is_err());
}

struct OverreportInput;

impl AsyncInput for OverreportInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<io::Result<usize>> {
        panic!("the safe poll_read override must be used")
    }

    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(output.len() + 1))
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
fn test_async_buffered_input_async_buffered_overreport_fill_more() {
    let mut wrapper = Box::pin(AsyncBufferedInput::with_capacity(OverreportInput, 4));
    assert_invalid_data(wrapper.as_mut().poll_fill_more(&mut context()));
    assert_eq!(0, wrapper.unread_len());
    assert!(wrapper.unread().is_empty());
}

#[test]
fn test_async_buffered_input_async_buffered_overreport_refill_and_direct() {
    for requested in [1, 4] {
        let mut wrapper = Box::pin(AsyncBufferedInput::with_capacity(OverreportInput, 4));
        let mut output = [0; 4];
        let result = unsafe {
            wrapper
                .as_mut()
                .poll_read_unchecked(&mut context(), &mut output, 0, requested)
        };
        assert_invalid_data(result);
        assert_eq!(0, wrapper.unread_len());
        assert!(wrapper.unread().is_empty());
    }
}

struct PinnedInputState {
    bytes: Vec<u8>,
    position: usize,
    pending: usize,
}

struct PinnedInput {
    state: Rc<RefCell<PinnedInputState>>,
    address: Rc<Cell<Option<usize>>>,
    polls: Rc<Cell<usize>>,
    _pinned: PhantomPinned,
}

impl PinnedInput {
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

impl AsyncInput for PinnedInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        self.as_ref().observe();
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        if state.pending > 0 {
            state.pending -= 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let read = count.min(state.bytes.len() - state.position);
        output[index..index + read].copy_from_slice(&state.bytes[state.position..state.position + read]);
        state.position += read;
        Poll::Ready(Ok(read))
    }
}

fn pinned_input(pending: usize) -> PinnedInput {
    PinnedInput {
        state: Rc::new(RefCell::new(PinnedInputState {
            bytes: b"abcdef".to_vec(),
            position: 0,
            pending,
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

fn drive_input<T>(mut poll: impl FnMut(&mut Context<'_>) -> Poll<io::Result<T>>) -> T {
    let woke = Arc::new(AtomicBool::new(false));
    let waker = Waker::from(Arc::new(FlagWake(woke.clone())));
    let mut cx = Context::from_waker(&waker);
    for _ in 0..64 {
        woke.store(false, Ordering::SeqCst);
        match poll(&mut cx) {
            Poll::Ready(Ok(value)) => return value,
            Poll::Ready(Err(error)) => panic!("pinned input failed: {error}"),
            Poll::Pending => assert!(woke.load(Ordering::SeqCst), "Pending must wake"),
        }
    }
    panic!("pinned input exceeded the poll limit")
}

#[test]
fn test_async_buffered_input_async_buffered_non_unpin_fill_more_and_until() {
    let inner = pinned_input(2);
    let state = inner.state.clone();
    let observer_address = inner.address.clone();
    let polls = inner.polls.clone();
    let mut wrapper = Box::pin(AsyncBufferedInput::with_capacity(inner, 4));
    let filled = drive_input(|cx| wrapper.as_mut().poll_fill_more(cx));
    assert!(filled);
    assert_eq!(b"abcd", wrapper.unread());
    // Consume a prefix so fill_until exercises compact plus another refill.
    let mut prefix = [0; 2];
    let consumed = drive_input(|cx| unsafe { wrapper.as_mut().poll_read_unchecked(cx, &mut prefix, 0, 2) });
    assert_eq!(2, consumed);
    assert_eq!(b"ab", &prefix);
    let full = drive_input(|cx| wrapper.as_mut().poll_fill_until(cx, 4));
    assert!(full);
    assert_eq!(b"cdef", wrapper.unread());
    assert_eq!(6, state.borrow().position);
    assert!(observer_address.get().is_some());
    assert!(polls.get() > 1);
}

#[test]
fn test_async_buffered_input_async_buffered_non_unpin_refill_and_direct() {
    let inner = pinned_input(1);
    let observer_address = inner.address.clone();
    let polls = inner.polls.clone();
    let mut wrapper = Box::pin(AsyncBufferedInput::with_capacity(inner, 4));
    let mut result = [0; 1];
    let read = drive_input(|cx| unsafe { wrapper.as_mut().poll_read_unchecked(cx, &mut result, 0, 1) });
    assert_eq!(1, read);
    assert_eq!(b"a", &result);
    assert_eq!(b"bcd", wrapper.unread());
    let mut consumed = [0; 3];
    let count = drive_input(|cx| unsafe { wrapper.as_mut().poll_read_unchecked(cx, &mut consumed, 0, 3) });
    assert_eq!(3, count);
    assert_eq!(b"bcd", &consumed);
    let mut direct = [0; 4];
    let read = drive_input(|cx| unsafe { wrapper.as_mut().poll_read_unchecked(cx, &mut direct, 0, 4) });
    assert_eq!(2, read);
    assert_eq!(b"ef", &direct[..read]);
    assert!(observer_address.get().is_some());
    assert!(polls.get() > 1);
}
