// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use qubit_io::{AsyncClose, AsyncOutput};

struct CloseOutput {
    /// Whether the output has been closed.
    closed: bool,
    /// Whether the next close poll should be pending.
    pending: bool,
}

impl AsyncOutput for CloseOutput {
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

impl AsyncClose for CloseOutput {
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.pending {
            self.pending = false;
            return Poll::Pending;
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_async_close_is_object_safe_and_default_future_closes_output() {
    let mut output = CloseOutput {
        closed: false,
        pending: false,
    };
    let mut cx = Context::from_waker(Waker::noop());
    {
        let mut future = output.close_async();
        match Future::poll(Pin::new(&mut future), &mut cx) {
            Poll::Ready(result) => result.expect("close should succeed"),
            Poll::Pending => panic!("close should complete"),
        }
    }
    assert!(output.closed);

    let mut output = std::pin::pin!(output);
    let mut output: Pin<&mut dyn AsyncClose<Item = u8>> = output.as_mut();
    assert!(AsyncClose::poll_close(output.as_mut(), &mut cx).is_ready());
}

#[test]
fn test_close_future_preserves_pending_before_completion() {
    let mut output = CloseOutput {
        closed: false,
        pending: true,
    };
    let mut future = output.close_async();
    let mut cx = Context::from_waker(Waker::noop());

    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_ready());
}
