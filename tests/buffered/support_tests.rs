// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;

pub(super) struct PrefixThenForbiddenInput {
    kind: ErrorKind,
    emitted: bool,
}

impl PrefixThenForbiddenInput {
    #[must_use]
    #[inline]
    pub(super) const fn new(kind: ErrorKind) -> Self {
        Self { kind, emitted: false }
    }
}

impl AsyncInput for PrefixThenForbiddenInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        panic!("the safe poll_read override must be used")
    }

    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        if self.emitted {
            Poll::Ready(Err(Error::new(self.kind, "buffered input prefix marker")))
        } else {
            self.emitted = true;
            output[0] = b'x';
            Poll::Ready(Ok(1))
        }
    }
}

pub(super) struct PrefixThenForbiddenOutput {
    kind: ErrorKind,
    accepted: Vec<u8>,
}

impl PrefixThenForbiddenOutput {
    #[must_use]
    #[inline]
    pub(super) const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            accepted: Vec::new(),
        }
    }

    #[must_use]
    #[inline]
    pub(super) fn accepted(&self) -> &[u8] {
        &self.accepted
    }
}

impl AsyncOutput for PrefixThenForbiddenOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        panic!("the safe poll_write override must be used")
    }

    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<std::io::Result<usize>> {
        if self.accepted.is_empty() {
            self.accepted.push(input[0]);
            Poll::Ready(Ok(1))
        } else {
            Poll::Ready(Err(Error::new(self.kind, "buffered output prefix marker")))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub(super) struct TestInput;

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

pub(super) struct TestOutput;

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
