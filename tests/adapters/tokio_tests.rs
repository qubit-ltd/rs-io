// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use qubit_io::{
    AsyncClose, AsyncInput, AsyncOutput, TokioAsyncRead, TokioAsyncWrite, TokioInput, TokioOutput,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct TokioReader {
    data: Vec<u8>,
    position: usize,
}

struct PendingTokioReader;

impl AsyncRead for PendingTokioReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
}

struct ErrorTokioReader;

impl AsyncRead for ErrorTokioReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(Error::new(ErrorKind::PermissionDenied, "read failed")))
    }
}

impl AsyncRead for TokioReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let remaining = &self.data[self.position..];
        let read = remaining.len().min(output.remaining());
        output.put_slice(&remaining[..read]);
        self.position += read;
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct TokioWriter {
    data: Vec<u8>,
    closed: bool,
}

impl AsyncWrite for TokioWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.data.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

struct QubitInput {
    data: Vec<u8>,
    position: usize,
}

impl AsyncInput for QubitInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        let remaining = &self.data[self.position..];
        let read = remaining.len().min(count);
        output[index..index + read].copy_from_slice(&remaining[..read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

struct PendingQubitInput;

impl AsyncInput for PendingQubitInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Pending
    }
}

struct ErrorQubitInput;

impl AsyncInput for ErrorQubitInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _output: &mut [u8],
        _index: usize,
        _count: usize,
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(ErrorKind::PermissionDenied, "read failed")))
    }
}

#[derive(Default)]
struct QubitOutput {
    data: Vec<u8>,
    closed: bool,
}

impl AsyncClose for QubitOutput {
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

impl AsyncOutput for QubitOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        self.data.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn context() -> Context<'static> {
    Context::from_waker(Waker::noop())
}

#[test]
fn test_tokio_types_adapt_to_qubit_async_io() {
    let mut input = TokioInput::new(TokioReader {
        data: vec![1, 2, 3],
        position: 0,
    });
    let mut output = [0_u8; 4];
    let mut cx = context();

    let read = AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut output)
        .expect_ready("Tokio reader should be ready")
        .expect("Tokio read should succeed");
    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0], output);

    let mut output = TokioOutput::new(TokioWriter::default());
    let written = AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, &[4, 5])
        .expect_ready("Tokio writer should be ready")
        .expect("Tokio write should succeed");
    assert_eq!(2, written);
    assert_eq!(&[4, 5], output.get_ref().data.as_slice());

    AsyncClose::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close should complete")
        .expect("close should succeed");
    assert!(output.get_ref().closed);
}

#[test]
fn test_qubit_types_adapt_to_tokio_async_io() {
    let mut input = TokioAsyncRead::new(QubitInput {
        data: vec![1, 2, 3],
        position: 0,
    });
    let mut bytes = [0_u8; 4];
    let mut buffer = ReadBuf::new(&mut bytes);
    let mut cx = context();

    AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut buffer)
        .expect_ready("Qubit input should be ready")
        .expect("Qubit input should read successfully");
    assert_eq!(&[1, 2, 3], buffer.filled());

    let mut output = TokioAsyncWrite::new(QubitOutput::default());
    let written = AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, &[4, 5])
        .expect_ready("Qubit output should be ready")
        .expect("Qubit output should write successfully");
    assert_eq!(2, written);
    assert_eq!(&[4, 5], output.get_ref().data.as_slice());
}

#[test]
fn test_tokio_adapters_preserve_zero_pending_and_error_reads() {
    let mut cx = context();
    let mut input = TokioInput::new(TokioReader {
        data: vec![1],
        position: 0,
    });
    let mut bytes = [0_u8; 1];

    // SAFETY: The empty range at index zero is valid.
    let read =
        unsafe { AsyncInput::poll_read_unchecked(Pin::new(&mut input), &mut cx, &mut bytes, 0, 0) }
            .expect_ready("zero read should complete")
            .expect("zero read should succeed");
    assert_eq!(0, read);

    let mut pending = TokioInput::new(PendingTokioReader);
    assert!(AsyncInput::poll_read(Pin::new(&mut pending), &mut cx, &mut bytes,).is_pending());

    let mut failed = TokioInput::new(ErrorTokioReader);
    let error = AsyncInput::poll_read(Pin::new(&mut failed), &mut cx, &mut bytes)
        .expect_ready("error should be ready")
        .expect_err("Tokio error should be preserved");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());

    let mut input = TokioAsyncRead::new(QubitInput {
        data: Vec::new(),
        position: 0,
    });
    let mut empty = [];
    let mut buffer = ReadBuf::new(&mut empty);
    AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut buffer)
        .expect_ready("empty Tokio read should complete")
        .expect("empty Tokio read should succeed");

    let mut pending = TokioAsyncRead::new(PendingQubitInput);
    let mut buffer = ReadBuf::new(&mut bytes);
    assert!(AsyncRead::poll_read(Pin::new(&mut pending), &mut cx, &mut buffer).is_pending());

    let mut failed = TokioAsyncRead::new(ErrorQubitInput);
    let mut buffer = ReadBuf::new(&mut bytes);
    let error = AsyncRead::poll_read(Pin::new(&mut failed), &mut cx, &mut buffer)
        .expect_ready("error should be ready")
        .expect_err("Qubit error should be preserved");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}

#[test]
fn test_tokio_adapter_accessors_and_flush_operations() {
    let mut cx = context();

    let mut input = TokioInput::new(TokioReader {
        data: vec![1],
        position: 0,
    });
    assert_eq!(0, input.get_ref().position);
    input.get_mut().position = 1;
    Pin::new(&mut input).get_pin_mut().get_mut().position = 0;
    assert_eq!(vec![1], input.into_inner().data);

    let mut output = TokioOutput::new(TokioWriter::default());
    output.get_mut().data.push(1);
    assert_eq!(&[1], output.get_ref().data.as_slice());
    Pin::new(&mut output).get_pin_mut().get_mut().data.push(2);
    AsyncOutput::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush should complete")
        .expect("flush should succeed");
    assert_eq!(vec![1, 2], output.into_inner().data);

    let mut input = TokioAsyncRead::new(QubitInput {
        data: vec![3],
        position: 0,
    });
    assert_eq!(0, input.get_ref().position);
    input.get_mut().position = 1;
    Pin::new(&mut input).get_pin_mut().get_mut().position = 0;
    assert_eq!(vec![3], input.into_inner().data);

    let mut output = TokioAsyncWrite::new(QubitOutput::default());
    output.get_mut().data.push(4);
    assert_eq!(&[4], output.get_ref().data.as_slice());
    Pin::new(&mut output).get_pin_mut().get_mut().data.push(5);
    AsyncWrite::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush should complete")
        .expect("flush should succeed");
    AsyncWrite::poll_shutdown(Pin::new(&mut output), &mut cx)
        .expect_ready("shutdown should complete")
        .expect("shutdown should succeed");
    assert!(output.get_ref().closed);
    assert_eq!(vec![4, 5], output.into_inner().data);
}

#[test]
fn test_tokio_output_adapter_accepts_empty_write() {
    let mut output = TokioOutput::new(TokioWriter::default());
    let mut cx = context();

    // SAFETY: The empty range at index zero is valid.
    let written =
        unsafe { AsyncOutput::poll_write_unchecked(Pin::new(&mut output), &mut cx, &[], 0, 0) }
            .expect_ready("empty write should complete")
            .expect("empty write should succeed");

    assert_eq!(0, written);
}

trait PollResultExt<T> {
    fn expect_ready(self, message: &str) -> T;
}

impl<T> PollResultExt<T> for Poll<T> {
    fn expect_ready(self, message: &str) -> T {
        match self {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("{message}"),
        }
    }
}
