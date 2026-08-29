// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::RefCell;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncClose;
use qubit_io::AsyncOutput;
use qubit_io::BoxAsyncOutput;

/// State recorded by the pinned asynchronous output.
#[derive(Default)]
struct OutputState {
    items: Vec<u8>,
    flushed: bool,
    closed: bool,
}

/// Non-`Unpin` byte output used to exercise pinned trait-object forwarding.
struct PinnedByteOutput {
    state: Rc<RefCell<OutputState>>,
    _pinned: PhantomPinned,
}

impl AsyncOutput for PinnedByteOutput {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        true
    }

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.state
            .borrow_mut()
            .items
            .extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.state.borrow_mut().flushed = true;
        Poll::Ready(Ok(()))
    }
}

impl AsyncClose for PinnedByteOutput {
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.state.borrow_mut().closed = true;
        Poll::Ready(Ok(()))
    }
}

/// Creates a task context backed by the no-op waker.
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

/// Forwards writes, flushes, and closes through a pinned boxed output object.
#[test]
fn test_box_async_output_forwards_pinned_trait_object_output() {
    let state = Rc::new(RefCell::new(OutputState::default()));
    let inner: Box<dyn AsyncClose<Item = u8>> = Box::new(PinnedByteOutput {
        state: Rc::clone(&state),
        _pinned: PhantomPinned,
    });
    let mut output = BoxAsyncOutput::new(inner);
    let mut context = context();

    assert!(output.is_buffered());
    assert!(output.get_ref().is_buffered());
    assert!(matches!(
        AsyncOutput::poll_write(Pin::new(&mut output), &mut context, &[4, 5]),
        Poll::Ready(Ok(2))
    ));
    assert!(matches!(
        AsyncOutput::poll_flush(Pin::new(&mut output), &mut context),
        Poll::Ready(Ok(()))
    ));
    assert!(matches!(
        AsyncClose::poll_close(Pin::new(&mut output), &mut context),
        Poll::Ready(Ok(()))
    ));

    let inner = output.into_inner();
    assert!(inner.as_ref().get_ref().is_buffered());

    let state = state.borrow();
    assert_eq!(&[4, 5], state.items.as_slice());
    assert!(state.flushed);
    assert!(state.closed);
}
