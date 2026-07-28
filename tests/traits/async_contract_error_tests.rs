// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error as StdError;
use std::io::{
    Error,
    ErrorKind,
};
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
    Waker,
};

use qubit_io::AsyncInput;

struct ForbiddenErrorInput {
    kind: ErrorKind,
}

impl AsyncInput for ForbiddenErrorInput {
    type Item = u8;

    /// Returns the configured forbidden asynchronous error.
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden async error")))
    }
}

/// Verifies that normalized asynchronous errors retain their source.
#[test]
fn test_async_contract_error_retains_original_error_context() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut input = ForbiddenErrorInput { kind };
        let mut output = [0_u8; 1];
        let mut cx = Context::from_waker(Waker::noop());
        let result =
            AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output);
        let Poll::Ready(Err(error)) = result else {
            panic!("forbidden async error should be returned");
        };

        assert_eq!(
            format!(
                "asynchronous I/O implementation returned {kind:?}: forbidden async error"
            ),
            error.to_string(),
        );
        let source = StdError::source(&error)
            .expect("contract error should retain its source")
            .downcast_ref::<Error>()
            .expect("contract error source should be an I/O error");
        assert_eq!(kind, source.kind());
        assert_eq!("forbidden async error", source.to_string());
    }
}
