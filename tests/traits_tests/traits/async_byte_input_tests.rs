// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncByteInput;
use qubit_io::AsyncInput;
use qubit_io::Input;

struct ItemInput<T> {
    items: VecDeque<T>,
    pending: bool,
    _pinned: PhantomPinned,
}

impl<T: Copy> ItemInput<T> {
    fn new(items: &[T]) -> Self {
        Self {
            items: items.iter().copied().collect(),
            pending: true,
            _pinned: PhantomPinned,
        }
    }
}

impl<T: Copy> Input for ItemInput<T> {
    type Item = T;

    unsafe fn read_unchecked(&mut self, output: &mut [T], index: usize, count: usize) -> io::Result<usize> {
        let read = count.min(self.items.len());
        for target in &mut output[index..index + read] {
            *target = self.items.pop_front().expect("read count must fit the input");
        }
        Ok(read)
    }
}

impl<T: Copy> AsyncInput for ItemInput<T> {
    type Item = T;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [T],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        // SAFETY: No field is structurally pinned or moved out of this value.
        let this = unsafe { self.get_unchecked_mut() };
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if this.pending {
            this.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        // SAFETY: The caller supplies a valid destination range.
        Poll::Ready(unsafe { Input::read_unchecked(this, output, index, count) })
    }
}

fn read<I: AsyncByteInput + ?Sized>(
    input: Pin<&mut I>,
    cx: &mut Context<'_>,
    output: &mut [u8],
) -> Poll<io::Result<usize>> {
    input.poll_read(cx, output)
}

#[test]
fn test_parameter_and_opaque_return_preserve_pinned_pending_reads() {
    fn input() -> impl AsyncByteInput {
        ItemInput::new(b"AB")
    }

    let mut input = Box::pin(input());
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = [0; 2];
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert_eq!(items, [0; 2]);
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, [b'A', b'B']);
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(0))));
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_specialized_bound() {
    let mut input = Box::pin(ItemInput::new(b"AB"));
    let input: Pin<&mut dyn AsyncInput<Item = u8>> = input.as_mut();
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = [0; 2];
    let mut input = input;
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, [b'A', b'B']);
}

#[test]
fn test_specialized_trait_object_supports_pinned_reads() {
    let mut input: Pin<Box<dyn AsyncByteInput>> = Box::pin(ItemInput::new(b"AB"));
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = [0; 2];
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, [b'A', b'B']);
}
