// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncChecksumOutput;
use qubit_io::AsyncOutput;

use super::support_tests::TestStream;

#[derive(Default)]
struct RecordingHasher {
    calls: Vec<Vec<u8>>,
}

impl Hasher for RecordingHasher {
    fn finish(&self) -> u64 {
        self.calls.len() as u64
    }
    fn write(&mut self, bytes: &[u8]) {
        self.calls.push(bytes.to_vec());
    }
}

#[derive(Default)]
struct VecOutput(Vec<u8>);

impl AsyncOutput for VecOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.0.extend_from_slice(&input[index..index + count]);
        Poll::Ready(Ok(count))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_async_checksum_output_starts_with_hasher_checksum() {
    let hasher = DefaultHasher::new();
    let expected = hasher.finish();
    let output = AsyncChecksumOutput::new(TestStream, hasher);
    assert_eq!(expected, output.checksum());
}

#[test]
fn test_async_checksum_output_rejects_safe_overreported_count() {
    let mut wrapper = AsyncChecksumOutput::new(super::support_tests::SafeOverreportOutput, DefaultHasher::new());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let before = wrapper.checksum();
    let result = AsyncOutput::poll_write(Pin::new(&mut wrapper), &mut cx, b"ab");
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(before, wrapper.checksum());
}

#[test]
fn test_async_checksum_output_preserves_chunks_and_skips_empty_writes() {
    let mut output = AsyncChecksumOutput::new(VecOutput::default(), RecordingHasher::default());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    assert!(matches!(
        Pin::new(&mut output).poll_write(&mut cx, b""),
        Poll::Ready(Ok(0))
    ));
    assert!(matches!(
        Pin::new(&mut output).poll_write(&mut cx, b"a"),
        Poll::Ready(Ok(1))
    ));
    assert!(matches!(
        Pin::new(&mut output).poll_write(&mut cx, b"bc"),
        Poll::Ready(Ok(2))
    ));
    assert_eq!(vec![b"a".to_vec(), b"bc".to_vec()], output.hasher().calls);
    assert_eq!(b"abc", output.inner().0.as_slice());
}
