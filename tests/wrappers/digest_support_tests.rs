// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Digest test fixtures: a real SHA-256 state without Clone and a pinned
//! stream.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use digest::FixedOutput;
use digest::HashMarker;
use digest::Output;
use digest::OutputSizeUser;
use digest::Update;
use qubit_io::AsyncClose;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use sha2::Sha256;

#[derive(Default)]
pub(crate) struct NonCloneDigest(Sha256);

impl OutputSizeUser for NonCloneDigest {
    type OutputSize = <Sha256 as OutputSizeUser>::OutputSize;
}

impl Update for NonCloneDigest {
    fn update(&mut self, bytes: &[u8]) {
        Update::update(&mut self.0, bytes);
    }
}

impl FixedOutput for NonCloneDigest {
    fn finalize_into(self, output: &mut Output<Self>) {
        FixedOutput::finalize_into(self.0, output);
    }
}

impl HashMarker for NonCloneDigest {}

pub(crate) enum Step {
    Transfer(usize),
    Pending,
    Error(ErrorKind),
    InvalidCount,
}

#[derive(Default)]
pub(crate) struct StreamState {
    pub(crate) bytes: Vec<u8>,
    pub(crate) position: usize,
    pub(crate) steps: VecDeque<Step>,
    pub(crate) flush_steps: VecDeque<Step>,
    pub(crate) close_steps: VecDeque<Step>,
    pub(crate) flush_calls: usize,
    pub(crate) close_calls: usize,
    pub(crate) buffered: bool,
}

// RefCell permits safe mutation through Pin<&Self>; PhantomPinned exercises
// wrappers without imposing an Unpin bound on their underlying streams.
#[derive(Default)]
pub(crate) struct PinnedStream {
    pub(crate) state: RefCell<StreamState>,
    _pin: PhantomPinned,
}

impl AsyncInput for PinnedStream {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        self.state.borrow().buffered
    }

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        match state.steps.pop_front().unwrap_or(Step::Transfer(count)) {
            Step::Transfer(maximum) => {
                let read = count.min(maximum).min(state.bytes.len() - state.position);
                output[index..index + read].copy_from_slice(&state.bytes[state.position..state.position + read]);
                state.position += read;
                Poll::Ready(Ok(read))
            }
            Step::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Step::Error(kind) => Poll::Ready(Err(io::Error::new(kind, "scripted read failure"))),
            Step::InvalidCount => Poll::Ready(Ok(count + 1)),
        }
    }
}

impl AsyncOutput for PinnedStream {
    type Item = u8;

    fn is_buffered(&self) -> bool {
        self.state.borrow().buffered
    }

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        match state.steps.pop_front().unwrap_or(Step::Transfer(count)) {
            Step::Transfer(maximum) => {
                let written = count.min(maximum);
                state.bytes.extend_from_slice(&input[index..index + written]);
                Poll::Ready(Ok(written))
            }
            Step::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Step::Error(kind) => Poll::Ready(Err(io::Error::new(kind, "scripted write failure"))),
            Step::InvalidCount => Poll::Ready(Ok(count + 1)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        state.flush_calls += 1;
        poll_control(state.flush_steps.pop_front(), cx)
    }
}

impl AsyncClose for PinnedStream {
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.as_ref().get_ref().state.borrow_mut();
        state.close_calls += 1;
        poll_control(state.close_steps.pop_front(), cx)
    }
}

fn poll_control(step: Option<Step>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    match step {
        Some(Step::Pending) => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
        Some(Step::Error(kind)) => Poll::Ready(Err(io::Error::new(kind, "scripted control failure"))),
        _ => Poll::Ready(Ok(())),
    }
}

pub(crate) fn ready<T>(result: Poll<io::Result<T>>) -> T {
    match result {
        Poll::Ready(result) => result.expect("successful ready result"),
        Poll::Pending => panic!("expected a ready result"),
    }
}

pub(crate) fn error_kind<T>(result: Poll<io::Result<T>>) -> ErrorKind {
    match result {
        Poll::Ready(Err(error)) => error.kind(),
        _ => panic!("expected a ready error"),
    }
}
