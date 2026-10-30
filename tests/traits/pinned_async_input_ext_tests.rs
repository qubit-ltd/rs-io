// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncInput;
use qubit_io::PinnedAsyncInputExt;

struct PinnedInput {
    _pinned: PhantomPinned,
}

impl AsyncInput for PinnedInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        output[index..index + count].fill(1);
        Poll::Ready(Ok(count))
    }
}

#[test]
fn test_pinned_async_input_ext_supports_non_unpin_trait_object() {
    let mut input = Box::pin(PinnedInput { _pinned: PhantomPinned });
    let mut input: Pin<&mut dyn AsyncInput<Item = u8>> = input.as_mut();
    let mut output = [0_u8; 2];
    let mut cx = Context::from_waker(Waker::noop());
    {
        output.fill(0);
        let mut future = input.read_async(&mut output);
        assert!(
            matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(2))),
            "single read should return the number of items read"
        );
    }
    assert_eq!([1, 1], output);
    {
        output.fill(0);
        let mut future = input.read_fully_async(&mut output);
        assert!(
            matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(2))),
            "read fully should return the number of items read"
        );
    }
    assert_eq!([1, 1], output);
    {
        output.fill(0);
        let mut future = input.read_exactly_async(&mut output);
        assert!(
            matches!(Future::poll(Pin::new(&mut future), &mut cx), Poll::Ready(Ok(()))),
            "exact read should complete after filling the destination"
        );
    }
    assert_eq!([1, 1], output);
}
