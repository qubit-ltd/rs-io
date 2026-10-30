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

use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io::Input;
use qubit_io::Output;

pub(super) struct ItemInput<T> {
    items: VecDeque<T>,
    pending: bool,
    _pinned: PhantomPinned,
}

impl<T: Copy> ItemInput<T> {
    #[must_use]
    pub(super) fn new(items: &[T]) -> Self {
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

pub(super) struct ItemOutput<'a, T> {
    items: &'a mut Vec<T>,
    pending: bool,
    _pinned: PhantomPinned,
}

impl<'a, T> ItemOutput<'a, T> {
    #[must_use]
    pub(super) fn new(items: &'a mut Vec<T>) -> Self {
        Self {
            items,
            pending: true,
            _pinned: PhantomPinned,
        }
    }
}

impl<T: Copy> Output for ItemOutput<'_, T> {
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
        Poll::Ready(unsafe { Output::write_unchecked(this, input, index, count) })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
