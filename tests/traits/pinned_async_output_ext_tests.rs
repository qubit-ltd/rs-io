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
use std::task::{Context, Poll, Waker};

use qubit_io::{AsyncClose, AsyncOutput, PinnedAsyncOutputExt};

struct PinnedOutput {
    closed: bool,
    _pinned: PhantomPinned,
}

impl AsyncClose for PinnedOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: This method does not move the pinned output.
        unsafe { self.get_unchecked_mut() }.closed = true;
        Poll::Ready(Ok(()))
    }
}

impl AsyncOutput for PinnedOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_pinned_async_output_ext_supports_non_unpin_trait_object() {
    let mut output = Box::pin(PinnedOutput {
        closed: false,
        _pinned: PhantomPinned,
    });
    let mut cx = Context::from_waker(Waker::noop());
    {
        let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
        let input = [1, 2];
        {
            let mut future = output.write_async(&input);
            assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());
        }
        {
            let mut future = output.write_fully_async(&input);
            assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());
        }
        {
            let mut future = output.flush_async();
            assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());
        }
        {
            let mut future = output.close_async();
            assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());
        }
    }
    assert!(output.closed);
}
