// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::io::Error;
use std::io::ErrorKind;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;

use futures_io::AsyncRead;
use futures_io::AsyncWrite;
use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io::FuturesAsyncRead;
use qubit_io::FuturesAsyncWrite;
use qubit_io::FuturesInput;
use qubit_io::FuturesOutput;

struct FuturesReader {
    data: Vec<u8>,
    position: usize,
}

struct PendingFuturesReader;

impl AsyncRead for PendingFuturesReader {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        Poll::Pending
    }
}

struct ErrorFuturesReader;

impl AsyncRead for ErrorFuturesReader {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(Error::new(ErrorKind::PermissionDenied, "read failed")))
    }
}

impl AsyncRead for FuturesReader {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let remaining = &self.data[self.position..];
        let read = remaining.len().min(output.len());
        output[..read].copy_from_slice(&remaining[..read]);
        self.position += read;
        Poll::Ready(Ok(read))
    }
}

#[derive(Default)]
struct FuturesWriter {
    data: Vec<u8>,
    closed: bool,
}

impl AsyncWrite for FuturesWriter {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<std::io::Result<usize>> {
        self.data.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
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

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct Gate {
    ready: bool,
    waker: Option<Waker>,
}
impl Gate {
    fn poll(&mut self, cx: &Context<'_>) -> Poll<()> {
        if self.ready {
            Poll::Ready(())
        } else {
            self.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn release(gate: &Arc<Mutex<Gate>>) {
    let waker = {
        let mut gate = gate.lock().expect("wake gate mutex");
        gate.ready = true;
        gate.waker.take().expect("pending operation must save its waker")
    };
    waker.wake();
}

struct WakeableFuturesReader {
    gate: Arc<Mutex<Gate>>,
    read: bool,
}

impl AsyncRead for WakeableFuturesReader {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, out: &mut [u8]) -> Poll<std::io::Result<usize>> {
        if self.gate.lock().expect("wake gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        if self.read || out.is_empty() {
            return Poll::Ready(Ok(0));
        }
        out[0] = b'a';
        self.read = true;
        Poll::Ready(Ok(1))
    }
}

struct WakeableQubitReader {
    gate: Arc<Mutex<Gate>>,
    read: bool,
}

impl AsyncInput for WakeableQubitReader {
    type Item = u8;
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        if self.gate.lock().expect("wake gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        if self.read || count == 0 {
            return Poll::Ready(Ok(0));
        }
        out[index] = b'a';
        self.read = true;
        Poll::Ready(Ok(1))
    }
}

#[test]
fn test_futures_read_bridges_save_wake_and_resume() {
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let gate = Arc::new(Mutex::new(Gate::default()));
    let mut input = FuturesInput::new(WakeableFuturesReader {
        gate: gate.clone(),
        read: false,
    });
    let mut bytes = [0; 2];
    assert!(AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut bytes).is_pending());
    release(&gate);
    assert_eq!(1, counter.0.load(Ordering::SeqCst));
    assert_eq!(
        1,
        AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
            .expect_ready("read resumes")
            .expect("read succeeds")
    );
    assert_eq!(b'a', bytes[0]);
    assert_eq!(
        0,
        AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
            .expect_ready("reader reaches EOF")
            .expect("EOF read succeeds")
    );
    let gate = Arc::new(Mutex::new(Gate::default()));
    let mut input = FuturesAsyncRead::new(WakeableQubitReader {
        gate: gate.clone(),
        read: false,
    });
    assert!(AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut bytes).is_pending());
    release(&gate);
    assert_eq!(
        1,
        AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
            .expect_ready("read resumes")
            .expect("read succeeds")
    );
    assert_eq!(b'a', bytes[0]);
    assert_eq!(
        0,
        AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
            .expect_ready("reader reaches EOF")
            .expect("EOF read succeeds")
    );
}

struct WakeableFuturesWriter {
    write_gate: Arc<Mutex<Gate>>,
    flush_gate: Arc<Mutex<Gate>>,
    close_gate: Arc<Mutex<Gate>>,
    data: Vec<u8>,
    closed: bool,
}

impl AsyncWrite for WakeableFuturesWriter {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, input: &[u8]) -> Poll<std::io::Result<usize>> {
        if self.write_gate.lock().expect("write gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        let n = input.len().min(1);
        self.data.extend_from_slice(&input[..n]);
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.flush_gate
            .lock()
            .expect("flush gate mutex")
            .poll(cx)
            .map(|()| Ok(()))
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.close_gate.lock().expect("close gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

struct WakeableQubitWriter {
    write_gate: Arc<Mutex<Gate>>,
    flush_gate: Arc<Mutex<Gate>>,
    close_gate: Arc<Mutex<Gate>>,
    data: Vec<u8>,
    closed: bool,
}

impl AsyncOutput for WakeableQubitWriter {
    type Item = u8;
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        if self.write_gate.lock().expect("write gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        let n = count.min(1);
        self.data.extend_from_slice(&input[index..index + n]);
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.flush_gate
            .lock()
            .expect("flush gate mutex")
            .poll(cx)
            .map(|()| Ok(()))
    }
}

impl AsyncClose for WakeableQubitWriter {
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.close_gate.lock().expect("close gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_futures_write_bridges_save_wake_and_resume_each_stage() {
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let make_gate = || Arc::new(Mutex::new(Gate::default()));
    let (write_gate, flush_gate, close_gate) = (make_gate(), make_gate(), make_gate());
    let mut output = FuturesOutput::new(WakeableFuturesWriter {
        write_gate: write_gate.clone(),
        flush_gate: flush_gate.clone(),
        close_gate: close_gate.clone(),
        data: Vec::new(),
        closed: false,
    });
    assert!(AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, b"ab").is_pending());
    release(&write_gate);
    assert_eq!(
        1,
        AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, b"ab")
            .expect_ready("write resumes")
            .expect("write succeeds")
    );
    assert_eq!(b"a", output.get_ref().data.as_slice());
    assert!(AsyncOutput::poll_flush(Pin::new(&mut output), &mut cx).is_pending());
    release(&flush_gate);
    AsyncOutput::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush resumes")
        .expect("flush succeeds");
    assert!(AsyncClose::poll_close(Pin::new(&mut output), &mut cx).is_pending());
    release(&close_gate);
    AsyncClose::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close resumes")
        .expect("close succeeds");
    assert!(output.get_ref().closed);
    let (write_gate, flush_gate, close_gate) = (make_gate(), make_gate(), make_gate());
    let mut output = FuturesAsyncWrite::new(WakeableQubitWriter {
        write_gate: write_gate.clone(),
        flush_gate: flush_gate.clone(),
        close_gate: close_gate.clone(),
        data: Vec::new(),
        closed: false,
    });
    assert!(AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, b"xy").is_pending());
    release(&write_gate);
    assert_eq!(
        1,
        AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, b"xy")
            .expect_ready("write resumes")
            .expect("write succeeds")
    );
    assert_eq!(b"x", output.get_ref().data.as_slice());
    assert!(AsyncWrite::poll_flush(Pin::new(&mut output), &mut cx).is_pending());
    release(&flush_gate);
    AsyncWrite::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush resumes")
        .expect("flush succeeds");
    assert!(AsyncWrite::poll_close(Pin::new(&mut output), &mut cx).is_pending());
    release(&close_gate);
    AsyncWrite::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close resumes")
        .expect("close succeeds");
    assert!(output.get_ref().closed);
    let mut output = FuturesAsyncWrite::new(QubitOutput::default());
    assert_eq!(
        0,
        AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, b"")
            .expect_ready("empty write is ready")
            .expect("empty write succeeds")
    );
}

struct PinnedFuturesInput {
    address: Cell<usize>,
    position: usize,
    _pin: PhantomPinned,
}

impl PinnedFuturesInput {
    fn check(&self) {
        let address = self as *const Self as usize;
        let old = self.address.replace(address);
        assert!(old == 0 || old == address, "pinned futures input moved");
    }
}

impl AsyncRead for PinnedFuturesInput {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut [u8]) -> Poll<std::io::Result<usize>> {
        // SAFETY: only ordinary fields are accessed; the !Unpin inner remains in its
        // pinned box.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        if this.position == 0 && !output.is_empty() {
            output[0] = b'p';
            this.position = 1;
            return Poll::Ready(Ok(1));
        }
        Poll::Ready(Ok(0))
    }
}

impl AsyncInput for PinnedFuturesInput {
    type Item = u8;
    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: only ordinary fields are accessed; the !Unpin inner remains in its
        // pinned box.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        if this.position == 0 && count > 0 {
            output[index] = b'p';
            this.position = 1;
            return Poll::Ready(Ok(1));
        }
        Poll::Ready(Ok(0))
    }
}

struct PinnedFuturesOutput {
    address: Cell<usize>,
    data: Vec<u8>,
    _pin: PhantomPinned,
}

impl PinnedFuturesOutput {
    fn check(&self) {
        let address = self as *const Self as usize;
        let old = self.address.replace(address);
        assert!(old == 0 || old == address, "pinned futures output moved");
    }
}

impl AsyncWrite for PinnedFuturesOutput {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<std::io::Result<usize>> {
        // SAFETY: only an ordinary Vec field is changed; the pinned value is not moved.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        this.data.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned value is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned value is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}

impl AsyncOutput for PinnedFuturesOutput {
    type Item = u8;
    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<std::io::Result<usize>> {
        // SAFETY: only an ordinary Vec field is changed; the pinned value is not moved.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        this.data.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned value is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}

impl AsyncClose for PinnedFuturesOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned value is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_futures_bridges_support_pinned_inner_values() {
    let mut cx = context();
    let mut input = Box::pin(FuturesInput::new(PinnedFuturesInput {
        address: Cell::new(0),
        position: 0,
        _pin: PhantomPinned,
    }));
    let mut bytes = [0; 1];
    assert_eq!(
        1,
        AsyncInput::poll_read(input.as_mut(), &mut cx, &mut bytes)
            .expect_ready("pinned input ready")
            .expect("read succeeds")
    );
    let _ = AsyncInput::poll_read(input.as_mut(), &mut cx, &mut bytes)
        .expect_ready("pinned EOF ready")
        .expect("read succeeds");
    let mut input = Box::pin(FuturesAsyncRead::new(PinnedFuturesInput {
        address: Cell::new(0),
        position: 0,
        _pin: PhantomPinned,
    }));
    assert_eq!(
        1,
        AsyncRead::poll_read(input.as_mut(), &mut cx, &mut bytes)
            .expect_ready("pinned futures read ready")
            .expect("read succeeds")
    );
    let mut output = Box::pin(FuturesOutput::new(PinnedFuturesOutput {
        address: Cell::new(0),
        data: Vec::new(),
        _pin: PhantomPinned,
    }));
    assert_eq!(
        2,
        AsyncOutput::poll_write(output.as_mut(), &mut cx, b"ab")
            .expect_ready("pinned output ready")
            .expect("write succeeds")
    );
    AsyncOutput::poll_flush(output.as_mut(), &mut cx)
        .expect_ready("pinned flush ready")
        .expect("flush succeeds");
    AsyncClose::poll_close(output.as_mut(), &mut cx)
        .expect_ready("pinned close ready")
        .expect("close succeeds");
    let mut output = Box::pin(FuturesAsyncWrite::new(PinnedFuturesOutput {
        address: Cell::new(0),
        data: Vec::new(),
        _pin: PhantomPinned,
    }));
    assert_eq!(
        2,
        AsyncWrite::poll_write(output.as_mut(), &mut cx, b"cd")
            .expect_ready("pinned futures write ready")
            .expect("write succeeds")
    );
    AsyncWrite::poll_flush(output.as_mut(), &mut cx)
        .expect_ready("pinned futures flush ready")
        .expect("flush succeeds");
    AsyncWrite::poll_close(output.as_mut(), &mut cx)
        .expect_ready("pinned futures close ready")
        .expect("close succeeds");
}

#[test]
fn test_futures_types_adapt_to_qubit_async_io() {
    let mut input = FuturesInput::new(FuturesReader {
        data: vec![1, 2, 3],
        position: 0,
    });
    let mut bytes = [0_u8; 4];
    let mut cx = context();

    let read = AsyncInput::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
        .expect_ready("futures reader should be ready")
        .expect("futures read should succeed");
    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0], bytes);

    let mut output = FuturesOutput::new(FuturesWriter::default());
    let written = AsyncOutput::poll_write(Pin::new(&mut output), &mut cx, &[4, 5])
        .expect_ready("futures writer should be ready")
        .expect("futures write should succeed");
    assert_eq!(2, written);
    assert_eq!(&[4, 5], output.get_ref().data.as_slice());

    AsyncClose::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close should complete")
        .expect("close should succeed");
    assert!(output.get_ref().closed);
}

#[test]
fn test_qubit_types_adapt_to_futures_async_io() {
    let mut input = FuturesAsyncRead::new(QubitInput {
        data: vec![1, 2, 3],
        position: 0,
    });
    let mut bytes = [0_u8; 4];
    let mut cx = context();

    let read = AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut bytes)
        .expect_ready("Qubit input should be ready")
        .expect("Qubit input should read successfully");
    assert_eq!(3, read);
    assert_eq!([1, 2, 3, 0], bytes);

    let mut output = FuturesAsyncWrite::new(QubitOutput::default());
    let written = AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, &[4, 5])
        .expect_ready("Qubit output should be ready")
        .expect("Qubit output should write successfully");
    assert_eq!(2, written);
    assert_eq!(&[4, 5], output.get_ref().data.as_slice());
}

#[test]
fn test_futures_adapter_accessors_and_flush_operations() {
    let mut cx = context();

    let mut input = FuturesInput::new(FuturesReader {
        data: vec![1],
        position: 0,
    });
    assert_eq!(0, input.get_ref().position);
    input.get_mut().position = 1;
    Pin::new(&mut input).get_pin_mut().get_mut().position = 0;
    assert_eq!(vec![1], input.into_inner().data);

    let mut output = FuturesOutput::new(FuturesWriter::default());
    output.get_mut().data.push(1);
    assert_eq!(&[1], output.get_ref().data.as_slice());
    Pin::new(&mut output).get_pin_mut().get_mut().data.push(2);
    AsyncOutput::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush should complete")
        .expect("flush should succeed");
    assert_eq!(vec![1, 2], output.into_inner().data);

    let mut input = FuturesAsyncRead::new(QubitInput {
        data: vec![3],
        position: 0,
    });
    assert_eq!(0, input.get_ref().position);
    input.get_mut().position = 1;
    Pin::new(&mut input).get_pin_mut().get_mut().position = 0;
    assert_eq!(vec![3], input.into_inner().data);

    let mut output = FuturesAsyncWrite::new(QubitOutput::default());
    output.get_mut().data.push(4);
    assert_eq!(&[4], output.get_ref().data.as_slice());
    Pin::new(&mut output).get_pin_mut().get_mut().data.push(5);
    AsyncWrite::poll_flush(Pin::new(&mut output), &mut cx)
        .expect_ready("flush should complete")
        .expect("flush should succeed");
    AsyncWrite::poll_close(Pin::new(&mut output), &mut cx)
        .expect_ready("close should complete")
        .expect("close should succeed");
    assert!(output.get_ref().closed);
    assert_eq!(vec![4, 5], output.into_inner().data);
}

#[test]
fn test_futures_input_preserves_zero_pending_and_error_reads() {
    let mut cx = context();
    let mut bytes = [0_u8; 1];
    let mut input = FuturesInput::new(FuturesReader {
        data: vec![1],
        position: 0,
    });

    // SAFETY: The empty range at index zero is valid.
    let read = unsafe { AsyncInput::poll_read_unchecked(Pin::new(&mut input), &mut cx, &mut bytes, 0, 0) }
        .expect_ready("zero read should complete")
        .expect("zero read should succeed");
    assert_eq!(0, read);

    let mut pending = FuturesInput::new(PendingFuturesReader);
    assert!(AsyncInput::poll_read(Pin::new(&mut pending), &mut cx, &mut bytes).is_pending());

    let mut failed = FuturesInput::new(ErrorFuturesReader);
    let error = AsyncInput::poll_read(Pin::new(&mut failed), &mut cx, &mut bytes)
        .expect_ready("error should be ready")
        .expect_err("futures-io error should be preserved");
    assert_eq!(ErrorKind::PermissionDenied, error.kind());

    let mut input = FuturesAsyncRead::new(QubitInput {
        data: Vec::new(),
        position: 0,
    });
    let read = AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut [])
        .expect_ready("empty read should complete")
        .expect("empty read should succeed");
    assert_eq!(0, read);
}

#[test]
fn test_futures_output_adapter_accepts_empty_write() {
    let mut output = FuturesOutput::new(FuturesWriter::default());
    let mut cx = context();

    // SAFETY: The empty range at index zero is valid.
    let written = unsafe { AsyncOutput::poll_write_unchecked(Pin::new(&mut output), &mut cx, &[], 0, 0) }
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
