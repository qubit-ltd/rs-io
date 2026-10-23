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

/// Input that emits an optional valid prefix, then reports one item too many.
pub(super) struct SafeOverreportedInput {
    prefix: usize,
    prefix_emitted: bool,
}

impl SafeOverreportedInput {
    pub(super) const fn new(prefix: usize) -> Self {
        Self {
            prefix,
            prefix_emitted: false,
        }
    }
}

impl AsyncInput for SafeOverreportedInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        unreachable!("test must exercise the safe override")
    }

    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if !this.prefix_emitted && this.prefix > 0 {
            let n = this.prefix.min(output.len());
            output[..n].fill(0xA5);
            this.prefix_emitted = true;
            return Poll::Ready(Ok(n));
        }
        Poll::Ready(Ok(output.len() + 1))
    }
}

/// Output that accepts an optional valid prefix, then reports one too many.
pub(super) struct SafeOverreportedOutput {
    prefix: usize,
    prefix_emitted: bool,
    pub(super) accepted: Vec<u8>,
}

impl SafeOverreportedOutput {
    pub(super) fn new(prefix: usize) -> Self {
        Self {
            prefix,
            prefix_emitted: false,
            accepted: Vec::new(),
        }
    }
}

impl AsyncOutput for SafeOverreportedOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        _count: usize,
    ) -> Poll<Result<usize>> {
        unreachable!("test must exercise the safe override")
    }

    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if !this.prefix_emitted && this.prefix > 0 {
            let n = this.prefix.min(input.len());
            this.accepted.extend_from_slice(&input[..n]);
            this.prefix_emitted = true;
            return Poll::Ready(Ok(n));
        }
        Poll::Ready(Ok(input.len() + 1))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
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

/// Emits distinct bytes one at a time and optionally yields once after a
/// prefix.
pub(super) struct CancellationInput {
    pub(super) bytes: Vec<u8>,
    pub(super) position: usize,
    pub(super) pending_after: Option<usize>,
}

impl AsyncInput for CancellationInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if this.pending_after == Some(this.position) {
            this.pending_after = None;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        if this.position == this.bytes.len() {
            return Poll::Ready(Ok(0));
        }
        output[index] = this.bytes[this.position];
        this.position += 1;
        Poll::Ready(Ok(1))
    }
}

/// Records actual accepted bytes, optionally yielding once after a prefix.
pub(super) struct CancellationOutput {
    pub(super) bytes: Vec<u8>,
    pub(super) pending_after: Option<usize>,
}

impl AsyncOutput for CancellationOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if this.pending_after == Some(this.bytes.len()) {
            this.pending_after = None;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        this.bytes.push(input[index]);
        Poll::Ready(Ok(1))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}
