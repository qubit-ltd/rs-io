// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Cursor,
    Error,
    ErrorKind,
};
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
    Waker,
};

use futures_io::AsyncRead;
use qubit_io::{
    AsyncInput,
    FuturesInput,
};

struct ErrorReader(ErrorKind);

impl AsyncRead for ErrorReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buffer: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(self.0, "invalid async read error")))
    }
}

/// Creates a task context backed by the no-op waker.
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_futures_input_type_is_public() {
    assert!(
        std::any::type_name::<FuturesInput<Cursor<Vec<u8>>>>()
            .contains("FuturesInput")
    );
}

/// Tests that the futures-io input adapter rejects forbidden error kinds.
#[test]
fn test_futures_input_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = FuturesInput::new(ErrorReader(kind));
        let mut buffer = [0_u8; 1];
        let mut cx = context();

        // SAFETY: The requested range covers the one-element destination.
        let result = unsafe {
            AsyncInput::poll_read_unchecked(
                Pin::new(&mut input),
                &mut cx,
                &mut buffer,
                0,
                1,
            )
        };
        let Poll::Ready(Err(error)) = result else {
            panic!("forbidden futures-io read error should be ready");
        };
        assert_eq!(ErrorKind::InvalidData, error.kind());
    }
}
