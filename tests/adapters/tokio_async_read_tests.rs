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

use qubit_io::AsyncInput;
use qubit_io::TokioAsyncRead;
use tokio::io::AsyncRead;
use tokio::io::ReadBuf;

#[test]
fn test_tokio_async_read_type_is_public() {
    assert!(std::any::type_name::<TokioAsyncRead<Cursor<Vec<u8>>>>().contains("TokioAsyncRead"));
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
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.1 += 1;
        match self.0 {
            Action::Count(n) => {
                let initialized = n.min(output.len());
                output[..initialized].fill(9);
                Poll::Ready(Ok(n))
            }
            Action::Error(k) => Poll::Ready(Err(Error::new(k, "inner marker"))),
            Action::Pending => Poll::Pending,
        }
    }
}
fn poll(action: Action, initial: &[u8], capacity: usize) -> (Poll<std::io::Result<()>>, Vec<u8>, usize) {
    let mut reader = TokioAsyncRead::new(Reader(action, 0));
    let mut storage = vec![0; capacity];
    storage[..initial.len()].copy_from_slice(initial);
    let mut output = ReadBuf::new(&mut storage);
    output.advance(initial.len());
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let result = AsyncRead::poll_read(Pin::new(&mut reader), &mut cx, &mut output);
    (result, output.filled().to_vec(), reader.get_ref().1)
}

#[test]
fn test_tokio_async_read_validates_count_and_preserves_filled_prefix() {
    assert!(matches!(poll(Action::Count(1), &[5], 3), (Poll::Ready(Ok(())), ref filled, 1) if filled == &[5, 9]));
    assert!(matches!(poll(Action::Count(2), &[], 1).0, Poll::Ready(Err(ref e)) if e.kind() == ErrorKind::InvalidData));
    assert!(matches!(poll(Action::Count(3), &[5], 3).0, Poll::Ready(Err(ref e)) if e.kind() == ErrorKind::InvalidData));
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let (result, filled, calls) = poll(Action::Error(kind), &[5], 3);
        let error = match result {
            Poll::Ready(Err(e)) => e,
            other => panic!("expected error, got {other:?}"),
        };
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        assert_eq!(filled, [5]);
        assert_eq!(calls, 1);
        let source = std::error::Error::source(&error);
        assert!(
            source
                .and_then(|e| e.downcast_ref::<Error>())
                .is_some_and(|e| e.kind() == kind)
        );
    }
    let (result, filled, _) = poll(Action::Error(ErrorKind::Other), &[5], 3);
    assert!(matches!(result, Poll::Ready(Err(ref e)) if e.kind() == ErrorKind::Other));
    assert_eq!(filled, [5]);
    let (result, filled, _) = poll(Action::Pending, &[5], 3);
    assert!(matches!(result, Poll::Pending));
    assert_eq!(filled, [5]);
    let (result, filled, calls) = poll(Action::Pending, &[5, 6], 2);
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(filled, [5, 6]);
    assert_eq!(calls, 0);
}
