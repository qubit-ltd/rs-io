// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::type_name;
use std::error::Error as StdError;
use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_io::AsyncRead;
use qubit_io::AsyncInput;
use qubit_io::FuturesAsyncRead;

#[test]
fn test_futures_async_read_type_is_public() {
    assert!(type_name::<FuturesAsyncRead<Cursor<Vec<u8>>>>().contains("FuturesAsyncRead"));
}

#[derive(Clone, Copy)]
enum Action {
    Count(usize),
    Error(ErrorKind),
    Pending,
}

struct Reader(Action, usize);

impl AsyncInput for Reader {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        panic!("safe poll_read override must be used")
    }

    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.1 += 1;
        match self.0 {
            Action::Count(count) => Poll::Ready(Ok(count)),
            Action::Error(kind) => Poll::Ready(Err(Error::new(kind, "inner marker"))),
            Action::Pending => Poll::Pending,
        }
    }
}

fn poll(action: Action, output: &mut [u8]) -> (Poll<std::io::Result<usize>>, usize) {
    let mut reader = FuturesAsyncRead::new(Reader(action, 0));
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    let result = AsyncRead::poll_read(Pin::new(&mut reader), &mut cx, output);
    (result, reader.get_ref().1)
}

#[test]
fn test_futures_async_read_validates_count_and_errors() {
    let mut output = [0; 2];
    let (result, calls) = poll(Action::Count(1), &mut output);
    assert!(matches!(result, Poll::Ready(Ok(1))));
    assert_eq!(calls, 1);
    let error = match poll(Action::Count(3), &mut output).0 {
        Poll::Ready(Err(error)) => error,
        other => panic!("expected error, got {other:?}"),
    };
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let error = match poll(Action::Error(kind), &mut output).0 {
            Poll::Ready(Err(error)) => error,
            other => panic!("expected error, got {other:?}"),
        };
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        let source = StdError::source(&error);
        assert!(
            source
                .and_then(|source| source.downcast_ref::<Error>())
                .is_some_and(|e| e.kind() == kind)
        );
    }
    let error = match poll(Action::Error(ErrorKind::Other), &mut output).0 {
        Poll::Ready(Err(error)) => error,
        other => panic!("expected error, got {other:?}"),
    };
    assert_eq!(error.kind(), ErrorKind::Other);
    assert!(matches!(poll(Action::Pending, &mut output).0, Poll::Pending));
    let mut empty = [];
    let (result, calls) = poll(Action::Count(3), &mut empty);
    assert!(matches!(result, Poll::Ready(Ok(0))));
    assert_eq!(calls, 0);
}
