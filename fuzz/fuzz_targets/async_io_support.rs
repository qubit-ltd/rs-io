// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Test doubles and bounded drivers used by the asynchronous I/O fuzz target.

use std::future::Future;
use std::future::poll_fn;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::pin::Pin;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use qubit_io::AsyncBufferedInput;
use qubit_io::AsyncBufferedOutput;
use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;

/// Runs every bounded asynchronous I/O scenario for one fuzz input.
pub(super) fn run(data: &[u8]) {
    fuzz_buffered_input(data);
    fuzz_buffered_output(data);
    fuzz_forbidden_errors(data);
    fuzz_buffered_state_sequence(data);
}

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
        output[index..index + read].copy_from_slice(&self.bytes[self.position..self.position + read]);
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

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
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
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Err(Error::new(self.kind, "forbidden flush error")))
    }
}

impl AsyncClose for ForbiddenOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
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

/// Runs a bounded sequence of buffered input and output state transitions.
fn fuzz_buffered_state_sequence(data: &[u8]) {
    if data.len() < 3 {
        return;
    }
    let capacity = usize::from(data[0] % 16) + 1;
    let chunk_size = usize::from(data[1] % 8) + 1;
    let sequence = &data[2..];
    let midpoint = sequence.len() / 2;
    let (operations, source) = sequence.split_at(midpoint);
    let max_steps = operations.len().min(128);
    let wake = Arc::new(ObservedWake::default());
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);

    let inner = ChunkedInput {
        bytes: source,
        position: 0,
        chunk_size,
        pending: false,
        enable_pending: true,
    };
    let mut input = AsyncBufferedInput::with_capacity(inner, capacity);
    let mut cursor = 0;
    let mut input_done = false;
    let mut accepted = Vec::new();
    let mut output = AsyncBufferedOutput::with_capacity(
        ChunkedOutput {
            bytes: Vec::new(),
            chunk_size,
            write_pending: false,
            flush_pending: false,
            close_pending: false,
            enable_pending: true,
            closed: false,
        },
        capacity,
    );
    let mut output_closed = false;

    for (step, operation) in operations.iter().copied().take(max_steps).enumerate() {
        wake.0.store(false, Ordering::SeqCst);
        let kind = operation % 8;
        let cancel_once = operation & 0x80 != 0;
        match kind {
            0 => {
                let request = usize::from(operation % capacity as u8) + 1;
                let mut buffer = vec![0; request];
                let mut future = input.read_async(&mut buffer);
                let first = Future::poll(Pin::new(&mut future), &mut context);
                let result = match first {
                    Poll::Ready(result) => Some(result),
                    Poll::Pending if cancel_once => {
                        assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                        drop(future);
                        None
                    }
                    Poll::Pending => {
                        let mut ready = None;
                        for _ in 0..256 {
                            match Future::poll(Pin::new(&mut future), &mut context) {
                                Poll::Ready(result) => {
                                    ready = Some(result);
                                    break;
                                }
                                Poll::Pending => {
                                    assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake")
                                }
                            }
                        }
                        drop(future);
                        Some(ready.expect("bounded read should complete"))
                    }
                };
                if let Some(result) = result {
                    let read = result.expect("scripted input should not fail");
                    assert_eq!(&source[cursor..cursor + read], &buffer[..read]);
                    cursor += read;
                    input_done = read == 0;
                }
            }
            1 if !input_done => {
                let filled = if cancel_once {
                    match Pin::new(&mut input).poll_fill_more(&mut context) {
                        Poll::Pending => {
                            assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                            None
                        }
                        Poll::Ready(result) => Some(result),
                    }
                } else {
                    Some(
                        drive_observed(
                            poll_fn(|cx| Pin::new(&mut input).poll_fill_more(cx)),
                            256,
                            &mut context,
                            &wake,
                        )
                        .expect("bounded refill should complete"),
                    )
                };
                if let Some(result) = filled {
                    match result {
                        Ok(filled) => input_done = !filled,
                        Err(error)
                            if error.kind() == ErrorKind::InvalidInput && input.unread_len() == input.capacity() => {}
                        Err(error) => panic!("scripted refill failed unexpectedly: {error}"),
                    }
                }
            }
            2 if !input_done => {
                let request = usize::from(operation % capacity as u8) + 1;
                let filled = if cancel_once {
                    match Pin::new(&mut input).poll_fill_until(&mut context, request) {
                        Poll::Pending => {
                            assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                            None
                        }
                        Poll::Ready(result) => Some(result),
                    }
                } else {
                    Some(
                        drive_observed(
                            poll_fn(|cx| Pin::new(&mut input).poll_fill_until(cx, request)),
                            256,
                            &mut context,
                            &wake,
                        )
                        .expect("bounded fill_until should complete"),
                    )
                };
                if let Some(result) = filled {
                    input_done = !result.expect("scripted fill_until should not fail");
                }
            }
            3 => {
                let count = usize::from(operation >> 5).min(input.unread_len());
                assert!(count <= input.unread_len());
                // SAFETY: `count` is bounded by the current unread window above.
                unsafe { input.consume(count) };
                cursor += count;
            }
            4 if !output_closed => {
                let byte = source.get(step % source.len().max(1)).copied().unwrap_or_default();
                let input = [byte];
                let mut future = output.write_async(&input);
                let first = Future::poll(Pin::new(&mut future), &mut context);
                match first {
                    Poll::Ready(Ok(written)) => accepted.extend_from_slice(&input[..written]),
                    Poll::Ready(Err(error)) => panic!("scripted write failed: {error}"),
                    Poll::Pending if cancel_once => {
                        assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                        drop(future);
                    }
                    Poll::Pending => {
                        let result =
                            drive_observed(future, 256, &mut context, &wake).expect("bounded write should complete");
                        let written = result.expect("scripted write should not fail");
                        accepted.extend_from_slice(&input[..written]);
                    }
                }
            }
            5 if !output_closed => {
                let count = usize::from(operation % capacity as u8) + 1;
                if cancel_once {
                    let result = Pin::new(&mut output).poll_ensure_spare_capacity(&mut context, count);
                    if result.is_pending() {
                        assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                    }
                } else {
                    drive_observed(
                        poll_fn(|cx| Pin::new(&mut output).poll_ensure_spare_capacity(cx, count)),
                        256,
                        &mut context,
                        &wake,
                    )
                    .expect("bounded capacity operation should complete")
                    .expect("scripted capacity operation should not fail");
                }
            }
            6 if !output_closed => {
                if cancel_once {
                    let mut flush = output.flush_async();
                    if Future::poll(Pin::new(&mut flush), &mut context).is_pending() {
                        assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                        drop(flush);
                    } else {
                        drop(flush);
                    }
                } else {
                    drive_observed(output.flush_async(), 256, &mut context, &wake)
                        .expect("bounded flush should complete")
                        .expect("scripted flush should not fail");
                }
            }
            7 if !output_closed => {
                if cancel_once {
                    let mut close = output.close_async();
                    match Future::poll(Pin::new(&mut close), &mut context) {
                        Poll::Pending => {
                            assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake");
                        }
                        Poll::Ready(Ok(())) => output_closed = true,
                        Poll::Ready(Err(error)) => {
                            panic!("scripted close should not fail: {error}");
                        }
                    }
                    drop(close);
                } else {
                    drive_observed(output.close_async(), 256, &mut context, &wake)
                        .expect("bounded close should complete")
                        .expect("scripted close should not fail");
                    output_closed = true;
                }
            }
            _ => {}
        }

        let buffered_end = cursor + input.unread_len();
        assert!(buffered_end <= input.inner().position);
        assert_eq!(&source[cursor..buffered_end], input.unread());
        let mut remaining = input.unread().to_vec();
        remaining.extend_from_slice(&input.inner().bytes[input.inner().position..]);
        assert_eq!(&source[cursor..], remaining);
        let mut actual = output.inner().bytes.clone();
        actual.extend_from_slice(output.pending());
        assert_eq!(accepted, actual);
    }

    let (output_inner, pending) = output.into_parts();
    let mut recovered = output_inner.bytes;
    recovered.extend_from_slice(pending.readable());
    assert_eq!(accepted, recovered);

    let (input_inner, unread) = input.into_parts();
    let mut remaining = unread.readable().to_vec();
    remaining.extend_from_slice(&source[input_inner.position..]);
    assert_eq!(&source[cursor..], remaining);
}

#[derive(Default)]
struct ObservedWake(AtomicBool);

impl Wake for ObservedWake {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn drive_observed<F>(future: F, max_polls: usize, context: &mut Context<'_>, wake: &ObservedWake) -> Option<F::Output>
where
    F: Future,
{
    let mut future = pin!(future);
    for _ in 0..max_polls {
        match future.as_mut().poll(context) {
            Poll::Ready(output) => return Some(output),
            Poll::Pending => assert!(wake.0.swap(false, Ordering::SeqCst), "scripted Pending must wake"),
        }
    }
    None
}

/// Polls a self-waking test future with a caller-selected runaway guard.
fn drive<F>(future: F, max_polls: usize) -> Option<F::Output>
where
    F: Future,
{
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..max_polls {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return Some(output);
        }
    }
    None
}
