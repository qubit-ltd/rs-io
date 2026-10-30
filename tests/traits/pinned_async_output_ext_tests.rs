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

use qubit_io::AsyncClose;
use qubit_io::AsyncOutput;
use qubit_io::PinnedAsyncOutputExt;

struct PinnedOutput {
    bytes: Vec<u8>,
    closed: bool,
    flushes: usize,
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
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: This method does not move the pinned output.
        unsafe { self.get_unchecked_mut() }
            .bytes
            .extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: This method does not move the pinned output.
        unsafe { self.get_unchecked_mut() }.flushes += 1;
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_pinned_async_output_ext_supports_non_unpin_trait_object() {
    let mut output = Box::pin(PinnedOutput {
        bytes: Vec::new(),
        closed: false,
        flushes: 0,
        _pinned: PhantomPinned,
    });
    let mut cx = Context::from_waker(Waker::noop());
    let input = [1, 2];
    {
        let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
        {
            let mut future = output.write_async(&input);
            assert!(matches!(
                Future::poll(Pin::new(&mut future), &mut cx),
                Poll::Ready(Ok(2))
            ));
        }
    }
    assert_eq!([1, 2], output.as_ref().get_ref().bytes.as_slice());
    {
        let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
        {
            let mut future = output.write_fully_async(&input);
            assert!(matches!(
                Future::poll(Pin::new(&mut future), &mut cx),
                Poll::Ready(Ok(()))
            ));
        }
    }
    assert_eq!([1, 2, 1, 2], output.as_ref().get_ref().bytes.as_slice());
    {
        let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
        {
            let mut future = output.flush_async();
            assert!(matches!(
                Future::poll(Pin::new(&mut future), &mut cx),
                Poll::Ready(Ok(()))
            ));
        }
    }
    assert_eq!(1, output.as_ref().get_ref().flushes);
    {
        let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
        {
            let mut future = output.close_async();
            assert!(matches!(
                Future::poll(Pin::new(&mut future), &mut cx),
                Poll::Ready(Ok(()))
            ));
        }
    }
    assert!(output.as_ref().get_ref().closed, "close_async should close the output");
}
