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

use qubit_io::AsyncChecksumInput;
use qubit_io::AsyncInput;

use super::support_tests::TestStream;

/// Records each hasher write call so tests can inspect chunk boundaries.
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

struct SliceInput {
    bytes: &'static [u8],
    position: usize,
}

impl AsyncInput for SliceInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        let read = count.min(this.bytes.len() - this.position);
        output[index..index + read].copy_from_slice(&this.bytes[this.position..this.position + read]);
        this.position += read;
        Poll::Ready(Ok(read))
    }
}

#[test]
fn test_async_checksum_input_starts_with_hasher_checksum() {
    let hasher = DefaultHasher::new();
    let expected = hasher.finish();
    let input = AsyncChecksumInput::new(TestStream, hasher);
    assert_eq!(expected, input.checksum());
}

#[test]
fn test_async_checksum_input_rejects_safe_overreported_count() {
    let mut wrapper = AsyncChecksumInput::new(super::support_tests::SafeOverreportInput, DefaultHasher::new());
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let before = wrapper.checksum();
    let mut bytes = [0_u8; 2];
    let result = AsyncInput::poll_read(Pin::new(&mut wrapper), &mut cx, &mut bytes);
    assert!(matches!(result, Poll::Ready(Err(ref error)) if error.kind() == ErrorKind::InvalidData));
    assert_eq!(before, wrapper.checksum());
}

#[test]
fn test_async_checksum_input_preserves_chunks_and_skips_empty_reads() {
    let mut input = AsyncChecksumInput::new(
        SliceInput {
            bytes: b"abc",
            position: 0,
        },
        RecordingHasher::default(),
    );
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut empty = [];
    assert!(matches!(
        Pin::new(&mut input).poll_read(&mut cx, &mut empty),
        Poll::Ready(Ok(0))
    ));
    let mut first = [0; 1];
    assert!(matches!(
        Pin::new(&mut input).poll_read(&mut cx, &mut first),
        Poll::Ready(Ok(1))
    ));
    let mut rest = [0; 2];
    assert!(matches!(
        Pin::new(&mut input).poll_read(&mut cx, &mut rest),
        Poll::Ready(Ok(2))
    ));
    assert_eq!(vec![b"a".to_vec(), b"bc".to_vec()], input.hasher().calls);
}
