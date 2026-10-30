// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_io::AsyncWrite;
use qubit_io::AsyncClose;
use qubit_io::AsyncOutput;
use qubit_io::FuturesAsyncWrite;

#[test]
fn test_futures_async_write_type_is_public() {
    assert!(std::any::type_name::<FuturesAsyncWrite<Cursor<Vec<u8>>>>().contains("FuturesAsyncWrite"));
}

#[derive(Clone, Copy)]
enum Action {
    Count(usize),
    Error(ErrorKind),
    Pending,
}

struct Writer(Action, usize);

impl AsyncOutput for Writer {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        panic!("safe poll_write override must be used")
    }
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, _input: &[u8]) -> Poll<std::io::Result<usize>> {
        self.1 += 1;
        match self.0 {
            Action::Count(n) => Poll::Ready(Ok(n)),
            Action::Error(k) => Poll::Ready(Err(Error::new(k, "inner marker"))),
            Action::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncClose for Writer {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn poll(action: Action, input: &[u8]) -> (Poll<std::io::Result<usize>>, usize) {
    let mut writer = FuturesAsyncWrite::new(Writer(action, 0));
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let result = AsyncWrite::poll_write(Pin::new(&mut writer), &mut cx, input);
    (result, writer.get_ref().1)
}

#[test]
fn test_futures_async_write_validates_count_and_errors() {
    let (result, calls) = poll(Action::Count(1), b"ab");
    assert!(matches!(result, Poll::Ready(Ok(1))));
    assert_eq!(calls, 1);
    let error = match poll(Action::Count(3), b"ab").0 {
        Poll::Ready(Err(e)) => e,
        other => panic!("expected error, got {other:?}"),
    };
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let error = match poll(Action::Error(kind), b"ab").0 {
            Poll::Ready(Err(e)) => e,
            other => panic!("expected error, got {other:?}"),
        };
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        let source = std::error::Error::source(&error);
        assert!(
            source
                .and_then(|e| e.downcast_ref::<Error>())
                .is_some_and(|e| e.kind() == kind)
        );
    }
    let error = match poll(Action::Error(ErrorKind::Other), b"ab").0 {
        Poll::Ready(Err(e)) => e,
        other => panic!("expected error, got {other:?}"),
    };
    assert_eq!(error.kind(), ErrorKind::Other);
    assert!(matches!(poll(Action::Pending, b"ab").0, Poll::Pending));
}
