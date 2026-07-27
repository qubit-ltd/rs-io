// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Cursor, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use qubit_io::{AsyncInput, TokioInput};
use tokio::io::{AsyncRead, ReadBuf};

struct ErrorReader(ErrorKind);

impl AsyncRead for ErrorReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(self.0, "invalid async read error")))
    }
}

/// Creates a task context backed by the no-op waker.
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_tokio_input_type_is_public() {
    assert!(std::any::type_name::<TokioInput<Cursor<Vec<u8>>>>().contains("TokioInput"));
}

/// Tests that the Tokio input adapter rejects forbidden error kinds.
#[test]
fn test_tokio_input_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = TokioInput::new(ErrorReader(kind));
        let mut buffer = [0_u8; 1];
        let mut cx = context();

        // SAFETY: The requested range covers the one-element destination.
        let result = unsafe {
            AsyncInput::poll_read_unchecked(Pin::new(&mut input), &mut cx, &mut buffer, 0, 1)
        };
        let Poll::Ready(Err(error)) = result else {
            panic!("forbidden Tokio read error should be ready");
        };
        assert_eq!(ErrorKind::InvalidData, error.kind());
    }
}
