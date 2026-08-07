// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncInput;
use qubit_io::BoxAsyncInput;

/// Non-`Unpin` byte input used to exercise pinned trait-object forwarding.
struct PinnedByteInput {
    items: Vec<u8>,
    position: usize,
    _pinned: PhantomPinned,
}

impl AsyncInput for PinnedByteInput {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        true
    }

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        let remaining = this.items.len() - this.position;
        let read = remaining.min(count);
        output[index..index + read]
            .copy_from_slice(&this.items[this.position..this.position + read]);
        this.position += read;
        Poll::Ready(Ok(read))
    }
}

/// Creates a task context backed by the no-op waker.
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

/// Forwards reads through a pinned boxed asynchronous input trait object.
#[test]
fn test_box_async_input_forwards_pinned_trait_object_input() {
    let inner: Box<dyn AsyncInput<Item = u8>> = Box::new(PinnedByteInput {
        items: vec![1, 2, 3],
        position: 0,
        _pinned: PhantomPinned,
    });
    let mut input = BoxAsyncInput::new(inner);
    let mut context = context();
    let mut output = [0_u8; 2];

    assert!(input.is_buffered());
    assert!(input.get_ref().is_buffered());
    assert!(matches!(
        AsyncInput::poll_read(Pin::new(&mut input), &mut context, &mut output),
        Poll::Ready(Ok(2))
    ));
    assert_eq!([1, 2], output);

    let mut inner = input.into_inner();
    assert!(matches!(
        AsyncInput::poll_read(inner.as_mut(), &mut context, &mut output[..1]),
        Poll::Ready(Ok(1))
    ));
    assert_eq!(3, output[0]);
}
