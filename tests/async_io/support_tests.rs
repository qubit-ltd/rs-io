// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use qubit_io::{AsyncClose, AsyncInput, AsyncOutput};

pub struct TestInput;

impl AsyncInput for TestInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }
}

pub struct TestOutput;

impl AsyncOutput for TestOutput {
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

impl AsyncClose for TestOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub struct ForbiddenErrorOutput {
    kind: ErrorKind,
}

impl ForbiddenErrorOutput {
    pub const fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

impl AsyncOutput for ForbiddenErrorOutput {
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
        Poll::Ready(Err(Error::new(self.kind, "forbidden flush error")))
    }
}

impl AsyncClose for ForbiddenErrorOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden close error")))
    }
}
