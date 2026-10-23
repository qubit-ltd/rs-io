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
use std::io::Result;
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

use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io::TokioAsyncRead;
use qubit_io::TokioAsyncWrite;
use qubit_io::TokioInput;
use qubit_io::TokioOutput;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;

struct TokioReader {
    data: Vec<u8>,
    position: usize,
}

struct PendingTokioReader;

impl AsyncRead for PendingTokioReader {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }
}

struct ErrorTokioReader;

impl AsyncRead for ErrorTokioReader {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _output: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        Poll::Ready(Err(Error::new(ErrorKind::PermissionDenied, "read failed")))
    }
}

impl AsyncRead for TokioReader {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<Result<()>> {
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
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<Result<usize>> {
        self.data.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
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
    ) -> Poll<Result<usize>> {
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
    ) -> Poll<Result<usize>> {
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
    ) -> Poll<Result<usize>> {
        Poll::Ready(Err(Error::new(ErrorKind::PermissionDenied, "read failed")))
    }
}

#[derive(Default)]
struct QubitOutput {
    data: Vec<u8>,
    closed: bool,
}

impl AsyncClose for QubitOutput {
    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
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
    ) -> Poll<Result<usize>> {
        self.data.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
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

struct WakeableTokioReader {
    gate: Arc<Mutex<Gate>>,
    read: bool,
}
impl AsyncRead for WakeableTokioReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, out: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        let this = self.get_mut();
        if this.gate.lock().expect("wake gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        if !this.read {
            out.put_slice(b"a");
            this.read = true;
        }
        Poll::Ready(Ok(()))
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
    ) -> Poll<Result<usize>> {
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
fn test_tokio_read_bridges_save_wake_and_resume() {
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let gate = Arc::new(Mutex::new(Gate::default()));
    let mut input = TokioInput::new(WakeableTokioReader {
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
    let mut input = TokioAsyncRead::new(WakeableQubitReader {
        gate: gate.clone(),
        read: false,
    });
    let mut storage = [0; 2];
    let mut output = ReadBuf::new(&mut storage);
    assert!(AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut output).is_pending());
    release(&gate);
    let mut output = ReadBuf::new(&mut storage);
    AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut output)
        .expect_ready("read resumes")
        .expect("read succeeds");
    assert_eq!(b"a", output.filled());
    let mut output = ReadBuf::new(&mut storage);
    AsyncRead::poll_read(Pin::new(&mut input), &mut cx, &mut output)
        .expect_ready("Tokio reader reaches EOF")
        .expect("EOF read succeeds");
    assert!(output.filled().is_empty());
}

struct WakeableTokioWriter {
    write_gate: Arc<Mutex<Gate>>,
    flush_gate: Arc<Mutex<Gate>>,
    close_gate: Arc<Mutex<Gate>>,
    data: Vec<u8>,
    closed: bool,
}
impl AsyncWrite for WakeableTokioWriter {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, input: &[u8]) -> Poll<Result<usize>> {
        if self.write_gate.lock().expect("write gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        let n = input.len().min(1);
        self.data.extend_from_slice(&input[..n]);
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.flush_gate
            .lock()
            .expect("flush gate mutex")
            .poll(cx)
            .map(|()| Ok(()))
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
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
    ) -> Poll<Result<usize>> {
        if self.write_gate.lock().expect("write gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        let n = count.min(1);
        self.data.extend_from_slice(&input[index..index + n]);
        Poll::Ready(Ok(n))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.flush_gate
            .lock()
            .expect("flush gate mutex")
            .poll(cx)
            .map(|()| Ok(()))
    }
}
impl AsyncClose for WakeableQubitWriter {
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if self.close_gate.lock().expect("close gate mutex").poll(cx).is_pending() {
            return Poll::Pending;
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_tokio_write_bridges_save_wake_and_resume_each_stage() {
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let make_gate = || Arc::new(Mutex::new(Gate::default()));
    let (write_gate, flush_gate, close_gate) = (make_gate(), make_gate(), make_gate());
    let mut output = TokioOutput::new(WakeableTokioWriter {
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
    let mut output = TokioAsyncWrite::new(WakeableQubitWriter {
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
    assert!(AsyncWrite::poll_shutdown(Pin::new(&mut output), &mut cx).is_pending());
    release(&close_gate);
    AsyncWrite::poll_shutdown(Pin::new(&mut output), &mut cx)
        .expect_ready("shutdown resumes")
        .expect("shutdown succeeds");
    assert!(output.get_ref().closed);

    let mut output = TokioAsyncWrite::new(QubitOutput::default());
    assert_eq!(
        0,
        AsyncWrite::poll_write(Pin::new(&mut output), &mut cx, b"")
            .expect_ready("empty write is ready")
            .expect("empty write succeeds")
    );
}

struct PinnedTokioInput {
    address: Cell<usize>,
    position: usize,
    _pin: PhantomPinned,
}
impl PinnedTokioInput {
    fn check(&self) {
        let address = self as *const Self as usize;
        let old = self.address.replace(address);
        assert!(old == 0 || old == address, "pinned Tokio input moved");
    }
}
impl AsyncRead for PinnedTokioInput {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        // SAFETY: only unpinned fields are accessed; the !Unpin value remains in its
        // pinned box.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        if this.position == 0 && output.remaining() > 0 {
            output.put_slice(b"p");
            this.position = 1;
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncInput for PinnedTokioInput {
    type Item = u8;
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>> {
        // SAFETY: the pinned value and PhantomPinned field are never moved; only
        // ordinary fields are mutated.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        this.check();
        if this.position == 0 && count > 0 {
            output[index] = b'p';
            this.position = 1;
            return Poll::Ready(Ok(1));
        }
        Poll::Ready(Ok(0))
    }
}

#[test]
fn test_tokio_read_bridges_support_pinned_inner_values() {
    let mut cx = context();
    let inner = PinnedTokioInput {
        address: Cell::new(0),
        position: 0,
        _pin: PhantomPinned,
    };
    let mut input = Box::pin(TokioInput::new(inner));
    let mut bytes = [0; 1];
    assert_eq!(
        1,
        AsyncInput::poll_read(input.as_mut(), &mut cx, &mut bytes)
            .expect_ready("pinned input ready")
            .expect("read succeeds")
    );
    assert_eq!(b'p', bytes[0]);
    let _ = AsyncInput::poll_read(input.as_mut(), &mut cx, &mut bytes)
        .expect_ready("pinned EOF ready")
        .expect("read succeeds");

    let inner = PinnedTokioInput {
        address: Cell::new(0),
        position: 0,
        _pin: PhantomPinned,
    };
    let mut input = Box::pin(TokioAsyncRead::new(inner));
    let mut bytes = [0; 1];
    let mut buffer = ReadBuf::new(&mut bytes);
    AsyncRead::poll_read(input.as_mut(), &mut cx, &mut buffer)
        .expect_ready("pinned Tokio read ready")
        .expect("read succeeds");
    assert_eq!(b"p", buffer.filled());
}

struct PinnedTokioOutput {
    address: Cell<usize>,
    data: Vec<u8>,
    _pin: PhantomPinned,
}
impl PinnedTokioOutput {
    fn check(&self) {
        let address = self as *const Self as usize;
        let old = self.address.replace(address);
        assert!(old == 0 || old == address, "pinned Tokio output moved");
    }
}
impl AsyncWrite for PinnedTokioOutput {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8]) -> Poll<Result<usize>> {
        // SAFETY: the !Unpin inner stays in its pinned box; only its Vec field is
        // modified.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        this.data.extend_from_slice(input);
        Poll::Ready(Ok(input.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned inner is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned inner is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}
impl AsyncOutput for PinnedTokioOutput {
    type Item = u8;
    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<Result<usize>> {
        // SAFETY: the !Unpin inner stays in its pinned box; only its Vec field is
        // modified.
        let this = unsafe { self.get_unchecked_mut() };
        this.check();
        this.data.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned inner is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}
impl AsyncClose for PinnedTokioOutput {
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        // SAFETY: only an ordinary field is read; the pinned inner is not moved.
        unsafe { self.get_unchecked_mut() }.check();
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_tokio_write_bridges_support_pinned_inner_values() {
    let mut cx = context();
    let inner = PinnedTokioOutput {
        address: Cell::new(0),
        data: Vec::new(),
        _pin: PhantomPinned,
    };
    let mut output = Box::pin(TokioOutput::new(inner));
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
    let inner = PinnedTokioOutput {
        address: Cell::new(0),
        data: Vec::new(),
        _pin: PhantomPinned,
    };
    let mut output = Box::pin(TokioAsyncWrite::new(inner));
    assert_eq!(
        2,
        AsyncWrite::poll_write(output.as_mut(), &mut cx, b"cd")
            .expect_ready("pinned Tokio write ready")
            .expect("write succeeds")
    );
    AsyncWrite::poll_flush(output.as_mut(), &mut cx)
        .expect_ready("pinned Tokio flush ready")
        .expect("flush succeeds");
    AsyncWrite::poll_shutdown(output.as_mut(), &mut cx)
        .expect_ready("pinned Tokio shutdown ready")
        .expect("shutdown succeeds");
}

#[test]
fn test_tokio_duplex_round_trip_under_backpressure() {
    let counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&counter));
    let mut cx = Context::from_waker(&waker);
    let (sender, receiver) = tokio::io::duplex(1);
    let mut sender = TokioAsyncWrite::new(TokioOutput::new(sender));
    let mut receiver = TokioAsyncRead::new(TokioInput::new(receiver));
    let payload = b"duplex";
    assert_eq!(
        1,
        AsyncWrite::poll_write(Pin::new(&mut sender), &mut cx, &payload[..1])
            .expect_ready("first byte ready")
            .expect("first write succeeds")
    );
    assert!(AsyncWrite::poll_write(Pin::new(&mut sender), &mut cx, &payload[1..]).is_pending());
    let before = counter.0.load(Ordering::SeqCst);
    let mut first = [0; 1];
    let mut buffer = ReadBuf::new(&mut first);
    AsyncRead::poll_read(Pin::new(&mut receiver), &mut cx, &mut buffer)
        .expect_ready("buffered byte ready")
        .expect("first read succeeds");
    assert_eq!(&payload[..1], buffer.filled());
    assert!(
        counter.0.load(Ordering::SeqCst) > before,
        "freeing duplex capacity must wake the writer through both bridges"
    );
    let mut sent = 1;
    let mut received = first.to_vec();
    for _ in 0..32 {
        if sent < payload.len() {
            match AsyncWrite::poll_write(Pin::new(&mut sender), &mut cx, &payload[sent..]) {
                Poll::Ready(Ok(n)) => {
                    assert!(n > 0);
                    sent += n;
                }
                Poll::Pending => {}
                Poll::Ready(Err(error)) => panic!("duplex write failed: {error}"),
            }
        }
        let mut storage = [0; 2];
        let mut buffer = ReadBuf::new(&mut storage);
        match AsyncRead::poll_read(Pin::new(&mut receiver), &mut cx, &mut buffer) {
            Poll::Ready(Ok(())) => received.extend_from_slice(buffer.filled()),
            Poll::Pending => {}
            Poll::Ready(Err(error)) => panic!("duplex read failed: {error}"),
        }
        if received.len() == payload.len() {
            break;
        }
    }
    assert_eq!(payload.len(), sent);
    assert_eq!(payload, received.as_slice());
    AsyncWrite::poll_flush(Pin::new(&mut sender), &mut cx)
        .expect_ready("flush ready")
        .expect("flush succeeds");
    AsyncWrite::poll_shutdown(Pin::new(&mut sender), &mut cx)
        .expect_ready("shutdown ready")
        .expect("shutdown succeeds");
    let mut eof = [0; 1];
    let mut buffer = ReadBuf::new(&mut eof);
    AsyncRead::poll_read(Pin::new(&mut receiver), &mut cx, &mut buffer)
        .expect_ready("EOF ready")
        .expect("EOF read succeeds");
    assert!(buffer.filled().is_empty());
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
    let read = unsafe { AsyncInput::poll_read_unchecked(Pin::new(&mut input), &mut cx, &mut bytes, 0, 0) }
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
