// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::SeekFrom;

use qubit_io::Input;
use qubit_io::Output;
use qubit_io::Seekable;

/// Marker used by compile-time async wrapper surface tests.
pub(crate) struct TestStream;

/// Script selected for the next input operation.
pub(crate) enum InputAction<T> {
    /// Returns items from the stored sequence.
    Items(Vec<T>),
    /// Returns an error with the stored message.
    Error(&'static str),
    /// Violates the input contract by reporting too much progress.
    InvalidCount,
}

/// Configurable generic input used by synchronous wrapper tests.
pub(crate) struct ScriptedInput<T> {
    /// Script selected for reads.
    pub(crate) action: InputAction<T>,
    /// Buffering declaration returned by the input.
    pub(crate) buffered: bool,
    /// Logical position used by seek tests.
    pub(crate) position: u64,
    /// Optional seek failure.
    pub(crate) seek_error: Option<&'static str>,
}

impl<T> ScriptedInput<T> {
    /// Creates an input over `items`.
    pub(crate) fn items(items: Vec<T>) -> Self {
        Self {
            action: InputAction::Items(items),
            buffered: false,
            position: 0,
            seek_error: None,
        }
    }

    /// Creates an input that fails reads.
    pub(crate) fn failing(message: &'static str) -> Self {
        Self {
            action: InputAction::Error(message),
            buffered: false,
            position: 0,
            seek_error: None,
        }
    }

    /// Creates an input that reports an invalid count.
    pub(crate) fn invalid_count() -> Self {
        Self {
            action: InputAction::InvalidCount,
            buffered: false,
            position: 0,
            seek_error: None,
        }
    }

    /// Returns the number of unread scripted items.
    pub(crate) fn remaining_len(&self) -> usize {
        match &self.action {
            InputAction::Items(items) => items.len(),
            InputAction::Error(_) | InputAction::InvalidCount => 0,
        }
    }
}

impl<T> Input for ScriptedInput<T>
where
    T: Clone,
{
    type Item = T;

    fn is_buffered(&self) -> bool {
        self.buffered
    }

    unsafe fn read_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> io::Result<usize> {
        match &mut self.action {
            InputAction::Items(items) => {
                let read = items.len().min(count);
                output[index..index + read].clone_from_slice(&items[..read]);
                items.drain(..read);
                self.position += u64::try_from(read).expect("test count fits");
                Ok(read)
            }
            InputAction::Error(message) => Err(Error::other(*message)),
            InputAction::InvalidCount => Ok(count.saturating_add(1)),
        }
    }
}

impl<T> Seekable for ScriptedInput<T> {
    type Unit = T;

    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        if let Some(message) = self.seek_error {
            return Err(Error::other(message));
        }
        self.position = seek_position(self.position, position)?;
        Ok(self.position)
    }
}

/// Configurable generic output used by synchronous wrapper tests.
pub(crate) struct ScriptedOutput<T> {
    /// Items successfully accepted.
    pub(crate) items: Vec<T>,
    /// Maximum progress per write.
    pub(crate) max_chunk: usize,
    /// Optional write failure.
    pub(crate) write_error: Option<&'static str>,
    /// Optional flush failure.
    pub(crate) flush_error: Option<&'static str>,
    /// Whether writes report an invalid count.
    pub(crate) invalid_count: bool,
    /// Buffering declaration returned by the output.
    pub(crate) buffered: bool,
    /// Number of flush calls.
    pub(crate) flush_calls: usize,
    /// Logical position used by seek tests.
    pub(crate) position: u64,
    /// Optional seek failure.
    pub(crate) seek_error: Option<&'static str>,
}

impl<T> ScriptedOutput<T> {
    /// Creates an output that accepts all requested items.
    pub(crate) fn accepting() -> Self {
        Self::short(usize::MAX)
    }

    /// Creates an output with bounded progress per write.
    pub(crate) fn short(max_chunk: usize) -> Self {
        Self {
            items: Vec::new(),
            max_chunk,
            write_error: None,
            flush_error: None,
            invalid_count: false,
            buffered: false,
            flush_calls: 0,
            position: 0,
            seek_error: None,
        }
    }

    /// Creates an output that fails writes.
    pub(crate) fn failing_write(message: &'static str) -> Self {
        Self {
            write_error: Some(message),
            ..Self::accepting()
        }
    }

    /// Creates an output that fails flushes.
    pub(crate) fn failing_flush(message: &'static str) -> Self {
        Self {
            flush_error: Some(message),
            ..Self::accepting()
        }
    }

    /// Creates an output that reports an invalid count.
    pub(crate) fn invalid_count() -> Self {
        Self {
            invalid_count: true,
            ..Self::accepting()
        }
    }
}

impl<T> Output for ScriptedOutput<T>
where
    T: Clone,
{
    type Item = T;

    fn is_buffered(&self) -> bool {
        self.buffered
    }

    unsafe fn write_unchecked(&mut self, input: &[Self::Item], index: usize, count: usize) -> io::Result<usize> {
        if let Some(message) = self.write_error {
            return Err(Error::other(message));
        }
        if self.invalid_count {
            return Ok(count.saturating_add(1));
        }
        let written = count.min(self.max_chunk);
        self.items.extend_from_slice(&input[index..index + written]);
        self.position += u64::try_from(written).expect("test count fits");
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_calls += 1;
        if let Some(message) = self.flush_error {
            Err(Error::other(message))
        } else {
            Ok(())
        }
    }
}

impl<T> Seekable for ScriptedOutput<T> {
    type Unit = T;

    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        if let Some(message) = self.seek_error {
            return Err(Error::other(message));
        }
        self.position = seek_position(self.position, position)?;
        Ok(self.position)
    }
}

/// Resolves a test seek without requiring a backing collection.
fn seek_position(current: u64, position: SeekFrom) -> io::Result<u64> {
    match position {
        SeekFrom::Start(position) => Ok(position),
        SeekFrom::Current(offset) => {
            let target = i128::from(current) + i128::from(offset);
            u64::try_from(target).map_err(|_| Error::new(ErrorKind::InvalidInput, "negative seek"))
        }
        SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "end-relative seek is unsupported")),
    }
}

#[test]
fn test_wrapper_support_stream_is_constructible() {
    let _ = TestStream;
}

/// Input whose safe methods overreport while unchecked methods panic.
pub(crate) struct SafeOverreportInput;

impl Input for SafeOverreportInput {
    type Item = u8;

    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        Ok(output.len() + 1)
    }

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("safe override must be called")
    }
}

impl qubit_io::AsyncInput for SafeOverreportInput {
    type Item = u8;

    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        output: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(Ok(output.len() + 1))
    }

    unsafe fn poll_read_unchecked(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        _: &mut [u8],
        _: usize,
        _: usize,
    ) -> std::task::Poll<io::Result<usize>> {
        panic!("safe override must be called")
    }
}

/// Output whose safe methods overreport while unchecked methods panic.
pub(crate) struct SafeOverreportOutput;

impl Output for SafeOverreportOutput {
    type Item = u8;

    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        Ok(input.len() + 1)
    }

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("safe override must be called")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl qubit_io::AsyncOutput for SafeOverreportOutput {
    type Item = u8;

    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        input: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(Ok(input.len() + 1))
    }

    unsafe fn poll_write_unchecked(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        _: &[u8],
        _: usize,
        _: usize,
    ) -> std::task::Poll<io::Result<usize>> {
        panic!("safe override must be called")
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// Safe input that makes one valid short transfer before overreporting.
#[derive(Default)]
pub(crate) struct SafeProgressThenOverreportInput(std::cell::Cell<bool>);

impl Input for SafeProgressThenOverreportInput {
    type Item = u8;

    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.0.replace(true) {
            Ok(output.len() + 1)
        } else {
            Ok(output.len().min(1))
        }
    }

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("safe override must be called")
    }
}

impl qubit_io::AsyncInput for SafeProgressThenOverreportInput {
    type Item = u8;

    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        output: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = self.get_mut();
        let read = if this.0.replace(true) {
            output.len() + 1
        } else {
            output.len().min(1)
        };
        std::task::Poll::Ready(Ok(read))
    }

    unsafe fn poll_read_unchecked(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        _: &mut [u8],
        _: usize,
        _: usize,
    ) -> std::task::Poll<io::Result<usize>> {
        panic!("safe override must be called")
    }
}

/// Safe output that makes one valid short transfer before overreporting.
#[derive(Default)]
pub(crate) struct SafeProgressThenOverreportOutput(std::cell::Cell<bool>);

impl Output for SafeProgressThenOverreportOutput {
    type Item = u8;

    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.0.replace(true) {
            Ok(input.len() + 1)
        } else {
            Ok(input.len().min(1))
        }
    }

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("safe override must be called")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl qubit_io::AsyncOutput for SafeProgressThenOverreportOutput {
    type Item = u8;

    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        input: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let this = self.get_mut();
        let written = if this.0.replace(true) {
            input.len() + 1
        } else {
            input.len().min(1)
        };
        std::task::Poll::Ready(Ok(written))
    }

    unsafe fn poll_write_unchecked(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        _: &[u8],
        _: usize,
        _: usize,
    ) -> std::task::Poll<io::Result<usize>> {
        panic!("safe override must be called")
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}
