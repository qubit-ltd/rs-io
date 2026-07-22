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

use qubit_io::{
    AsyncOutput,
    TokioOutput,
};
use tokio::io::AsyncWrite;

struct ErrorWriter(ErrorKind);

impl AsyncWrite for ErrorWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(self.0, "invalid async write error")))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(self.0, "invalid async flush error")))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Creates a task context backed by the no-op waker.
fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_tokio_output_type_is_public() {
    assert!(
        std::any::type_name::<TokioOutput<Cursor<Vec<u8>>>>()
            .contains("TokioOutput")
    );
}

/// Tests that the Tokio output adapter rejects forbidden error kinds.
#[test]
fn test_tokio_output_rejects_forbidden_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut output = TokioOutput::new(ErrorWriter(kind));
        let mut cx = context();

        // SAFETY: The requested range covers the one-element source.
        let write_result = unsafe {
            AsyncOutput::poll_write_unchecked(
                Pin::new(&mut output),
                &mut cx,
                &[1],
                0,
                1,
            )
        };
        let Poll::Ready(Err(write_error)) = write_result else {
            panic!("forbidden Tokio write error should be ready");
        };
        assert_eq!(ErrorKind::InvalidData, write_error.kind());

        let flush_result =
            AsyncOutput::poll_flush(Pin::new(&mut output), &mut cx);
        let Poll::Ready(Err(flush_error)) = flush_result else {
            panic!("forbidden Tokio flush error should be ready");
        };
        assert_eq!(ErrorKind::InvalidData, flush_error.kind());
    }
}
