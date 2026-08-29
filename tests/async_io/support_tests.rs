// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;

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

impl AsyncClose for TestOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug)]
pub(super) enum PollResult {
    Pending,
    Read(usize),
    Write(usize),
    Error(Error),
}

#[derive(Debug)]
pub(super) struct ScriptedInput {
    steps: VecDeque<PollResult>,
}

impl ScriptedInput {
    pub(super) fn new(steps: impl IntoIterator<Item = PollResult>) -> Self {
        Self {
            steps: VecDeque::from_iter(steps),
        }
    }
}

impl AsyncInput for ScriptedInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        _index: usize,
        count: usize,
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();
        let step = this.steps.pop_front().unwrap_or(PollResult::Read(0));

        match step {
            PollResult::Pending => Poll::Pending,
            PollResult::Error(error) => Poll::Ready(Err(error)),
            PollResult::Read(read) => {
                let read = read.min(count).min(output.len());
                for value in output.iter_mut().take(read) {
                    *value = 0xA5;
                }
                Poll::Ready(Ok(read))
            }
            PollResult::Write(_) => Poll::Ready(Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid script: read requested, write provided",
            ))),
        }
    }
}

#[derive(Debug)]
pub(super) struct ScriptedOutput {
    steps: VecDeque<PollResult>,
}

impl ScriptedOutput {
    pub(super) fn new(steps: impl IntoIterator<Item = PollResult>) -> Self {
        Self {
            steps: VecDeque::from_iter(steps),
        }
    }
}

impl AsyncOutput for ScriptedOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();
        let step = this.steps.pop_front().unwrap_or(PollResult::Write(0));

        match step {
            PollResult::Pending => Poll::Pending,
            PollResult::Error(error) => Poll::Ready(Err(error)),
            PollResult::Write(written) => Poll::Ready(Ok(written)),
            PollResult::Read(_) => Poll::Ready(Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid script: write requested, read provided",
            ))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncClose for ScriptedOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub(super) struct ForbiddenErrorOutput {
    kind: ErrorKind,
}

impl ForbiddenErrorOutput {
    pub(super) const fn new(kind: ErrorKind) -> Self {
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
