// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::future::Future;
use std::io::Error;
use std::io::ErrorKind;
use std::io::{self};
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use libfuzzer_sys::fuzz_target;
use qubit_io::AsyncBufferedInput;
use qubit_io::AsyncBufferedOutput;
use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;

/// Keeps allocations and poll loops bounded outside the repository CI wrapper.
const MAX_FUZZ_INPUT_LEN: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(MAX_FUZZ_INPUT_LEN)];
    fuzz_buffered_input(data);
    fuzz_buffered_output(data);
    fuzz_forbidden_errors(data);
});

struct ChunkedInput<'a> {
    bytes: &'a [u8],
    position: usize,
    chunk_size: usize,
    pending: bool,
    enable_pending: bool,
}

impl AsyncInput for ChunkedInput<'_> {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if self.enable_pending && !self.pending {
            self.pending = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = false;
        let available = self.bytes.len() - self.position;
        let read = available.min(count).min(self.chunk_size);
        output[index..index + read]
            .copy_from_slice(&self.bytes[self.position..self.position + read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

struct ChunkedOutput {
    bytes: Vec<u8>,
    chunk_size: usize,
    write_pending: bool,
    flush_pending: bool,
    close_pending: bool,
    enable_pending: bool,
    closed: bool,
}

impl AsyncOutput for ChunkedOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        if self.enable_pending && !self.write_pending {
            self.write_pending = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.write_pending = false;
        let written = count.min(self.chunk_size);
        self.bytes.extend_from_slice(&input[index..index + written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.enable_pending && !self.flush_pending {
            self.flush_pending = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            self.flush_pending = false;
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncClose for ChunkedOutput {
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.enable_pending && !self.close_pending {
            self.close_pending = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.close_pending = false;
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

struct ForbiddenOutput {
    kind: ErrorKind,
}

impl AsyncOutput for ForbiddenOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(count))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden flush error")))
    }
}

impl AsyncClose for ForbiddenOutput {
    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden close error")))
    }
}

/// Exercises buffered reads across arbitrary partial-progress and pending
/// boundaries.
fn fuzz_buffered_input(data: &[u8]) {
    let configuration = data.first().copied().unwrap_or_default();
    let source = data.get(1..).unwrap_or_default();
    let inner = ChunkedInput {
        bytes: source,
        position: 0,
        chunk_size: usize::from(configuration % 8) + 1,
        pending: false,
        enable_pending: configuration & 0x80 != 0,
    };
    let capacity = usize::from(configuration % 16) + 1;
    let mut input = AsyncBufferedInput::with_capacity(inner, capacity);
    let mut output = vec![0_u8; source.len() + 1];
    let max_polls = source.len().saturating_mul(4).saturating_add(64);

    let read = drive(input.read_fully_async(&mut output), max_polls)
        .expect("bounded input future should complete")
        .expect("chunked input should not fail");

    assert_eq!(source.len(), read);
    assert_eq!(source, &output[..read]);
}

/// Exercises buffered writes, flush, and close across arbitrary partial
/// progress and pending boundaries.
fn fuzz_buffered_output(data: &[u8]) {
    let configuration = data.first().copied().unwrap_or_default();
    let source = data.get(1..).unwrap_or_default();
    let inner = ChunkedOutput {
        bytes: Vec::new(),
        chunk_size: usize::from(configuration % 8) + 1,
        write_pending: false,
        flush_pending: false,
        close_pending: false,
        enable_pending: configuration & 0x80 != 0,
        closed: false,
    };
    let capacity = usize::from(configuration % 16) + 1;
    let mut output = AsyncBufferedOutput::with_capacity(inner, capacity);
    let max_polls = source.len().saturating_mul(4).saturating_add(64);

    drive(output.write_fully_async(source), max_polls)
        .expect("bounded write future should complete")
        .expect("chunked output should not fail");
    drive(output.flush_async(), max_polls)
        .expect("bounded flush future should complete")
        .expect("chunked output flush should not fail");
    drive(output.close_async(), max_polls)
        .expect("bounded close future should complete")
        .expect("chunked output close should not fail");

    assert_eq!(source, output.inner().bytes.as_slice());
    assert!(output.inner().closed);
    assert_eq!(0, output.pending_len());
}

/// Verifies that safe flush and close futures reject forbidden error kinds.
fn fuzz_forbidden_errors(data: &[u8]) {
    let kind = if data.first().copied().unwrap_or_default() & 1 == 0 {
        ErrorKind::WouldBlock
    } else {
        ErrorKind::Interrupted
    };
    let mut flush_output = ForbiddenOutput { kind };
    let mut close_output = ForbiddenOutput { kind };

    let flush_error = drive(flush_output.flush_async(), 4)
        .expect("forbidden flush future should complete")
        .expect_err("forbidden flush error should be rejected");
    let close_error = drive(close_output.close_async(), 4)
        .expect("forbidden close future should complete")
        .expect_err("forbidden close error should be rejected");

    assert_eq!(ErrorKind::InvalidData, flush_error.kind());
    assert_eq!(ErrorKind::InvalidData, close_error.kind());
}

/// Polls a self-waking test future with a caller-selected runaway guard.
fn drive<F>(future: F, max_polls: usize) -> Option<F::Output>
where
    F: Future,
{
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..max_polls {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return Some(output);
        }
    }
    None
}
