// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::{
    collections::VecDeque,
    future::Future,
    io::{
        self,
        Error,
        ErrorKind,
    },
    pin::Pin,
    task::{
        Context,
        Poll,
        Waker,
    },
};

enum ReadStep {
    Data(Vec<u16>),
    Error(ErrorKind),
    Pending,
    Eof,
}

struct ScriptedInput {
    steps: VecDeque<ReadStep>,
    marker: usize,
}

impl ScriptedInput {
    fn new(steps: impl IntoIterator<Item = ReadStep>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            marker: 0,
        }
    }
}

impl AsyncInput for ScriptedInput {
    type Item = u16;

    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        match self.steps.pop_front().unwrap_or(ReadStep::Eof) {
            ReadStep::Data(values) => {
                let read = count.min(values.len());
                output[index..index + read].copy_from_slice(&values[..read]);
                Poll::Ready(Ok(read))
            }
            ReadStep::Error(kind) => {
                Poll::Ready(Err(Error::new(kind, "scripted read failure")))
            }
            ReadStep::Pending => Poll::Pending,
            ReadStep::Eof => Poll::Ready(Ok(0)),
        }
    }
}

enum WriteStep {
    Accept(usize),
    Error(ErrorKind),
    Pending,
    Zero,
}

enum FlushStep {
    Ready,
    Error(ErrorKind),
    Pending,
}

struct ScriptedOutput {
    values: Vec<u16>,
    write_steps: VecDeque<WriteStep>,
    flush_steps: VecDeque<FlushStep>,
    marker: usize,
    closed: bool,
    close_error: Option<ErrorKind>,
}

impl ScriptedOutput {
    fn new(
        write_steps: impl IntoIterator<Item = WriteStep>,
        flush_steps: impl IntoIterator<Item = FlushStep>,
    ) -> Self {
        Self {
            values: Vec::new(),
            write_steps: write_steps.into_iter().collect(),
            flush_steps: flush_steps.into_iter().collect(),
            marker: 0,
            closed: false,
            close_error: None,
        }
    }

    fn with_close_error(mut self, kind: ErrorKind) -> Self {
        self.close_error = Some(kind);
        self
    }
}

impl AsyncClose for ScriptedOutput {
    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(kind) = self.close_error.take() {
            return Poll::Ready(Err(Error::new(
                kind,
                "scripted close failure",
            )));
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

impl AsyncOutput for ScriptedOutput {
    type Item = u16;

    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        match self
            .write_steps
            .pop_front()
            .unwrap_or(WriteStep::Accept(count))
        {
            WriteStep::Accept(maximum) => {
                let written = count.min(maximum);
                self.values
                    .extend_from_slice(&input[index..index + written]);
                Poll::Ready(Ok(written))
            }
            WriteStep::Error(kind) => {
                Poll::Ready(Err(Error::new(kind, "scripted write failure")))
            }
            WriteStep::Pending => Poll::Pending,
            WriteStep::Zero => Poll::Ready(Ok(0)),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        match self.flush_steps.pop_front().unwrap_or(FlushStep::Ready) {
            FlushStep::Ready => Poll::Ready(Ok(())),
            FlushStep::Error(kind) => {
                Poll::Ready(Err(Error::new(kind, "scripted flush failure")))
            }
            FlushStep::Pending => Poll::Pending,
        }
    }
}

use qubit_io::{
    AsyncBufferedInput,
    AsyncBufferedOutput,
    AsyncClose,
    AsyncInput,
    AsyncOutput,
};

fn complete<F>(future: F) -> F::Output
where
    F: Future,
{
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    for _ in 0..256 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
    panic!("test future did not complete");
}

#[test]
fn test_async_buffered_input_preserves_generic_items_across_pending_refills()
-> io::Result<()> {
    let inner = ScriptedInput::new([
        ReadStep::Pending,
        ReadStep::Data(vec![10, 20]),
        ReadStep::Pending,
        ReadStep::Data(vec![30]),
    ]);
    let mut input = AsyncBufferedInput::with_capacity(inner, 4);
    let mut first = [0_u16; 1];
    let mut second = [0_u16; 3];

    assert!(input.is_buffered());
    assert_eq!(1, complete(input.read_async(&mut first))?);
    assert_eq!([10], first);
    assert_eq!(1, input.unread_len());
    assert_eq!(1, complete(input.read_async(&mut second))?);
    assert_eq!(20, second[0]);
    assert_eq!(1, complete(input.read_async(&mut second[1..]))?);
    assert_eq!([20, 30, 0], second);
    Ok(())
}

#[test]
fn test_async_buffered_output_retains_items_until_async_flush() -> io::Result<()>
{
    let inner = ScriptedOutput::new(
        [
            WriteStep::Pending,
            WriteStep::Accept(2),
            WriteStep::Accept(1),
        ],
        [],
    );
    let mut output = AsyncBufferedOutput::with_capacity(inner, 4);

    assert!(output.is_buffered());
    assert_eq!(3, complete(output.write_async(&[1, 2, 3]))?);
    assert!(output.inner().values.is_empty());
    assert_eq!(3, output.pending_len());

    complete(output.flush_async())?;
    assert_eq!(&[1, 2, 3], output.inner().values.as_slice());
    assert_eq!(0, output.pending_len());
    Ok(())
}

#[test]
fn test_async_buffered_input_exposes_owned_parts_and_accessors()
-> io::Result<()> {
    let inner = ScriptedInput::new([ReadStep::Data(vec![1, 2])]);
    let mut input = AsyncBufferedInput::new(inner);
    assert!(input.capacity() > 1);
    assert_eq!(0, input.inner().marker);
    input.inner_mut().marker = 1;

    let mut output = [0_u16; 1];
    assert_eq!(1, complete(input.read_async(&mut output))?);
    assert_eq!([1], output);
    assert_eq!(&[2], input.unread());

    let (inner, buffer) = input.into_parts();
    assert_eq!(1, inner.marker);
    assert_eq!(&[2], buffer.readable());

    let input = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Eof]),
        0,
    );
    assert_eq!(1, input.capacity());
    let (inner, buffer) = input.into_parts();
    assert_eq!(0, inner.marker);
    assert!(buffer.is_empty());
    Ok(())
}

#[test]
fn test_async_buffered_input_handles_empty_direct_pending_eof_and_error_reads()
{
    let mut cx = Context::from_waker(Waker::noop());
    let mut empty = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Data(vec![1])]),
        2,
    );
    let mut output = [];
    assert_eq!(
        0,
        Pin::new(&mut empty)
            .poll_read(&mut cx, &mut output)
            .expect_ready("empty read should be ready")
            .expect("empty read should succeed")
    );

    let mut direct = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Data(vec![3, 4, 5])]),
        2,
    );
    let mut output = [0_u16; 3];
    assert_eq!(
        3,
        Pin::new(&mut direct)
            .poll_read(&mut cx, &mut output)
            .expect_ready("direct read should be ready")
            .expect("direct read should succeed")
    );
    assert_eq!([3, 4, 5], output);
    assert_eq!(0, direct.unread_len());

    let mut pending = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Pending, ReadStep::Data(vec![6])]),
        2,
    );
    let mut output = [0_u16; 1];
    assert!(
        Pin::new(&mut pending)
            .poll_read(&mut cx, &mut output)
            .is_pending()
    );
    assert_eq!(
        1,
        Pin::new(&mut pending)
            .poll_read(&mut cx, &mut output)
            .expect_ready("second read should be ready")
            .expect("second read should succeed")
    );
    assert_eq!([6], output);

    let mut eof = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Eof]),
        2,
    );
    assert_eq!(
        0,
        Pin::new(&mut eof)
            .poll_read(&mut cx, &mut output)
            .expect_ready("end-of-input read should be ready")
            .expect("end-of-input read should succeed")
    );

    let mut failing = AsyncBufferedInput::with_capacity(
        ScriptedInput::new([ReadStep::Error(ErrorKind::PermissionDenied)]),
        2,
    );
    let error = Pin::new(&mut failing)
        .poll_read(&mut cx, &mut output)
        .expect_ready("scripted read error should be ready")
        .expect_err("scripted read should fail");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}

#[test]
fn test_async_buffered_output_exposes_pending_parts_and_direct_writes()
-> io::Result<()> {
    let inner = ScriptedOutput::new([], []);
    let mut output = AsyncBufferedOutput::new(inner);
    assert!(output.capacity() > 1);
    assert_eq!(0, output.inner().marker);
    output.inner_mut().marker = 1;
    assert_eq!(2, complete(output.write_async(&[1, 2]))?);
    assert_eq!(&[1, 2], output.pending());

    let (inner, buffer) = output.into_parts();
    assert_eq!(1, inner.marker);
    assert_eq!(&[1, 2], buffer.readable());

    let mut direct =
        AsyncBufferedOutput::with_capacity(ScriptedOutput::new([], []), 2);
    assert_eq!(3, complete(direct.write_async(&[3, 4, 5]))?);
    assert_eq!(&[3, 4, 5], direct.inner().values.as_slice());
    assert_eq!(0, direct.pending_len());

    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(
        0,
        Pin::new(&mut direct)
            .poll_write(&mut cx, &[])
            .expect_ready("empty write should be ready")
            .expect("empty write should succeed")
    );
    Ok(())
}

#[test]
fn test_async_buffered_output_writes_exact_capacity_directly_when_empty()
-> io::Result<()> {
    let mut output =
        AsyncBufferedOutput::with_capacity(ScriptedOutput::new([], []), 2);

    assert_eq!(2, complete(output.write_async(&[1, 2]))?);
    assert_eq!(&[1, 2], output.inner().values.as_slice());
    assert_eq!(0, output.pending_len());
    Ok(())
}

#[test]
fn test_async_buffered_output_buffers_write_that_exactly_fills_remaining_capacity()
-> io::Result<()> {
    let mut output =
        AsyncBufferedOutput::with_capacity(ScriptedOutput::new([], []), 2);

    assert_eq!(1, complete(output.write_async(&[1]))?);
    assert_eq!(1, complete(output.write_async(&[2]))?);
    assert!(output.inner().values.is_empty());
    assert_eq!(&[1, 2], output.pending());
    Ok(())
}

#[test]
fn test_async_buffered_output_handles_pending_and_partial_drains()
-> io::Result<()> {
    let inner = ScriptedOutput::new(
        [
            WriteStep::Pending,
            WriteStep::Accept(1),
            WriteStep::Accept(1),
            WriteStep::Accept(1),
        ],
        [FlushStep::Pending, FlushStep::Ready],
    );
    let mut output = AsyncBufferedOutput::with_capacity(inner, 2);
    assert_eq!(1, complete(output.write_async(&[1]))?);
    assert_eq!(1, complete(output.write_async(&[2]))?);

    let mut cx = Context::from_waker(Waker::noop());
    let input = [3];
    let mut future = output.write_async(&input);
    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    assert_eq!(
        1,
        Future::poll(Pin::new(&mut future), &mut cx)
            .expect_ready("draining write should complete")?
    );
    drop(future);
    assert_eq!(&[3], output.pending());

    let mut future = output.flush_async();
    assert!(Future::poll(Pin::new(&mut future), &mut cx).is_pending());
    Future::poll(Pin::new(&mut future), &mut cx)
        .expect_ready("inner flush should complete")?;
    drop(future);
    assert_eq!(&[1, 2, 3], output.inner().values.as_slice());
    assert_eq!(0, output.pending_len());
    Ok(())
}

#[test]
fn test_async_buffered_output_reports_drain_and_flush_failures()
-> io::Result<()> {
    let mut zero = AsyncBufferedOutput::with_capacity(
        ScriptedOutput::new([WriteStep::Zero], []),
        2,
    );
    assert_eq!(1, complete(zero.write_async(&[1]))?);
    let error = complete(zero.write_async(&[2, 3]))
        .expect_err("zero-length drain should fail");
    assert_eq!(ErrorKind::WriteZero, error.kind());
    assert_eq!(&[1], zero.pending());

    let mut failing_write = AsyncBufferedOutput::with_capacity(
        ScriptedOutput::new([WriteStep::Error(ErrorKind::BrokenPipe)], []),
        2,
    );
    assert_eq!(1, complete(failing_write.write_async(&[1]))?);
    let error = complete(failing_write.write_async(&[2, 3]))
        .expect_err("drain error should be preserved");
    assert_eq!(ErrorKind::BrokenPipe, error.kind());

    let mut failing_drain_flush = AsyncBufferedOutput::with_capacity(
        ScriptedOutput::new([WriteStep::Error(ErrorKind::ConnectionReset)], []),
        2,
    );
    assert_eq!(1, complete(failing_drain_flush.write_async(&[1]))?);
    let error = complete(failing_drain_flush.flush_async())
        .expect_err("flush should preserve its drain error");
    assert_eq!(ErrorKind::ConnectionReset, error.kind());

    let mut failing_inner_flush = AsyncBufferedOutput::with_capacity(
        ScriptedOutput::new([], [FlushStep::Error(ErrorKind::Other)]),
        1,
    );
    let error = complete(failing_inner_flush.flush_async())
        .expect_err("inner flush error should be preserved");
    assert_eq!(ErrorKind::Other, error.kind());
    Ok(())
}

#[test]
fn test_async_buffered_output_rejects_forbidden_flush_and_close_error_kinds() {
    for kind in [ErrorKind::WouldBlock, ErrorKind::Interrupted] {
        let mut flushing = AsyncBufferedOutput::new(ScriptedOutput::new(
            [],
            [FlushStep::Error(kind)],
        ));
        let mut closing = AsyncBufferedOutput::new(
            ScriptedOutput::new([], []).with_close_error(kind),
        );
        let mut cx = Context::from_waker(Waker::noop());

        let flush_error = Pin::new(&mut flushing)
            .poll_flush(&mut cx)
            .expect_ready("buffered flush error should be ready")
            .expect_err("buffered flush should reject a forbidden error");
        let close_error =
            AsyncClose::poll_close(Pin::new(&mut closing), &mut cx)
                .expect_ready("buffered close error should be ready")
                .expect_err("buffered close should reject a forbidden error");

        assert_eq!(ErrorKind::InvalidData, flush_error.kind());
        assert_eq!(ErrorKind::InvalidData, close_error.kind());
    }
}

#[test]
fn test_async_buffered_output_drains_before_closing_inner() -> io::Result<()> {
    let inner = ScriptedOutput::new([WriteStep::Accept(2)], []);
    let mut output = AsyncBufferedOutput::with_capacity(inner, 3);
    assert_eq!(2, complete(output.write_async(&[1, 2]))?);
    let mut cx = Context::from_waker(Waker::noop());

    AsyncClose::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close should complete")?;

    assert_eq!(&[1, 2], output.inner().values.as_slice());
    assert!(output.inner().closed);
    assert_eq!(0, output.pending_len());
    Ok(())
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
