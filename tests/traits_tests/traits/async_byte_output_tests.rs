// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncByteOutput;
use qubit_io::AsyncOutput;

struct ItemOutput<'a, T> {
    items: &'a mut Vec<T>,
    pending: bool,
    _pinned: PhantomPinned,
}

impl<'a, T> ItemOutput<'a, T> {
    fn new(items: &'a mut Vec<T>) -> Self {
        Self {
            items,
            pending: true,
            _pinned: PhantomPinned,
        }
    }
}

impl<T: Copy> qubit_io::Output for ItemOutput<'_, T> {
    type Item = T;

    unsafe fn write_unchecked(&mut self, input: &[T], index: usize, count: usize) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<T: Copy> AsyncOutput for ItemOutput<'_, T> {
    type Item = T;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[T],
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
        // SAFETY: The caller supplies a valid source range.
        Poll::Ready(unsafe { qubit_io::Output::write_unchecked(this, input, index, count) })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn write<O: AsyncByteOutput + ?Sized>(output: Pin<&mut O>, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
    output.poll_write(cx, b"AB")
}

#[test]
fn test_parameter_and_borrowed_opaque_return_preserve_pinned_pending_writes() {
    fn output(items: &mut Vec<u8>) -> impl AsyncByteOutput + '_ {
        ItemOutput::new(items)
    }

    let mut items = Vec::new();
    {
        let mut output = Box::pin(output(&mut items));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
        assert!(matches!(output.as_mut().poll_flush(&mut cx), Poll::Ready(Ok(()))));
    }
    assert_eq!(items, [b'A', b'B']);
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_specialized_bound() {
    let mut items = Vec::new();
    {
        let mut output = Box::pin(ItemOutput::new(&mut items));
        let mut output: Pin<&mut dyn AsyncOutput<Item = u8>> = output.as_mut();
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
    }
    assert_eq!(items, [b'A', b'B']);
}

#[test]
fn test_specialized_trait_object_supports_pinned_writes() {
    let mut items = Vec::new();
    {
        let mut output: Pin<Box<dyn AsyncByteOutput + '_>> = Box::pin(ItemOutput::new(&mut items));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
    }
    assert_eq!(items, [b'A', b'B']);
}
