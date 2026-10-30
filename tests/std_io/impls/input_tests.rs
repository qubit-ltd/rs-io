// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for the standard [`Read`](std::io::Read) bridge to [`Input`].

use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::io::IoSliceMut;
use std::io::Read;
#[cfg(debug_assertions)]
use std::panic::AssertUnwindSafe;
#[cfg(debug_assertions)]
use std::panic::catch_unwind;

use qubit_io::Input;

struct VectorStdReader;

impl Read for VectorStdReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("scalar read called")
    }
    fn read_vectored(&mut self, buffers: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        let mut bytes = b"abcd".iter();
        let mut total = 0;
        for buffer in buffers {
            for slot in buffer.iter_mut() {
                let Some(byte) = bytes.next() else {
                    return Ok(total);
                };
                *slot = *byte;
                total += 1;
            }
        }
        Ok(total)
    }
}

#[test]
fn test_vectored_std_reader_keeps_native_override() {
    let mut reader = VectorStdReader;
    let mut first = [0; 2];
    let mut second = [0; 2];
    assert_eq!(
        Input::read_vectored(&mut reader, &mut [&mut first[..], &mut second[..]]).unwrap(),
        4
    );
    assert_eq!((&first, &second), (b"ab", b"cd"));
}

struct EdgeStdReader {
    reply: Result<usize, ErrorKind>,
    calls: usize,
}

impl Read for EdgeStdReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("scalar read called")
    }

    fn read_vectored(&mut self, buffers: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.calls += 1;
        let count = self.reply.map_err(Error::from)?;
        let mut remaining = count;
        for buffer in buffers {
            for slot in buffer.iter_mut() {
                if remaining == 0 {
                    return Ok(count);
                }
                *slot = b'x';
                remaining -= 1;
            }
        }
        Ok(count)
    }
}

#[test]
fn test_vectored_std_reader_preserves_empty_short_error_and_count_contracts() {
    let mut reader = EdgeStdReader { reply: Ok(0), calls: 0 };
    assert_eq!(Input::read_vectored(&mut reader, &mut []).unwrap(), 0);
    assert_eq!(Input::read_vectored(&mut reader, &mut [&mut [], &mut []]).unwrap(), 0);
    assert_eq!(reader.calls, 0);

    let mut reader = EdgeStdReader { reply: Ok(1), calls: 0 };
    let mut first = [b'!'; 2];
    let mut second = [b'?'; 2];
    assert_eq!(
        Input::read_vectored(&mut reader, &mut [&mut first, &mut second]).unwrap(),
        1
    );
    assert_eq!(first, [b'x', b'!']);
    assert_eq!(second, [b'?', b'?']);
    assert_eq!(reader.calls, 1);

    for kind in [ErrorKind::Interrupted, ErrorKind::PermissionDenied] {
        let mut reader = EdgeStdReader {
            reply: Err(kind),
            calls: 0,
        };
        assert_eq!(
            Input::read_vectored(&mut reader, &mut [&mut [0; 1]])
                .unwrap_err()
                .kind(),
            kind
        );
        assert_eq!(reader.calls, 1);
    }

    let mut reader = EdgeStdReader { reply: Ok(5), calls: 0 };
    assert_eq!(
        Input::read_vectored(&mut reader, &mut [&mut [0; 2], &mut [0; 2]])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(reader.calls, 1);
}

struct OverreportingStdReader;

struct FailingStdReader;

impl Read for FailingStdReader {
    /// Returns a deterministic standard reader failure.
    fn read(&mut self, _output: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::PermissionDenied, "read failed"))
    }
}

impl Read for OverreportingStdReader {
    /// Deliberately violates the standard reader count contract.
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        Ok(output.len() + 1)
    }
}

/// Verifies normal and indexed reads through the standard reader bridge.
#[test]
fn test_read_blanket_impl_exposes_input_read_and_read_unchecked() {
    let mut cursor = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 4];

    let read = Input::read(&mut cursor, &mut output).expect("read should succeed");
    assert_eq!(2, read);
    assert_eq!(b"ab\x00\x00", &output);

    let mut cursor = Cursor::new(b"cd".to_vec());
    let mut output = [b'.'; 4];
    // SAFETY: output[1..3] is a valid destination range.
    let read = unsafe { Input::read_unchecked(&mut cursor, &mut output, 1, 2).expect("read_unchecked should succeed") };
    assert_eq!(2, read);
    assert_eq!(b".cd.", &output);
}

/// Verifies that the standard reader bridge rejects impossible read counts.
#[test]
fn test_read_blanket_impl_rejects_overreported_count() {
    let mut reader = OverreportingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output).expect_err("blanket input should validate the Read count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

/// Verifies that the standard reader bridge preserves reader errors.
#[test]
fn test_read_blanket_impl_propagates_std_read_error() {
    let mut reader = FailingStdReader;
    let mut output = [0_u8; 3];

    let error = Input::read(&mut reader, &mut output).expect_err("blanket input should propagate Read errors");

    assert_eq!(ErrorKind::PermissionDenied, error.kind());
}

#[cfg(debug_assertions)]
#[test]
fn test_read_blanket_impl_read_unchecked_panics_on_invalid_range() {
    let mut reader = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 1];

    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the range is intentionally invalid; this verifies
        // debug-assert behavior.
        let _ = unsafe { Input::read_unchecked(&mut reader, &mut output, 1, 2).expect("should panic") };
    }));

    assert!(result.is_err(), "read_unchecked should panic on invalid range");
}

#[cfg(debug_assertions)]
#[test]
fn test_read_blanket_impl_read_unchecked_panics_on_index_plus_count_overflow() {
    let mut reader = Cursor::new(b"ab".to_vec());
    let mut output = [0_u8; 1];

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = unsafe { Input::read_unchecked(&mut reader, &mut output, usize::MAX, 2).expect("should panic") };
    }));

    assert!(
        result.is_err(),
        "read_unchecked should panic when index + count overflows"
    );
}

#[test]
fn test_read_blanket_impl_read_unchecked_propagates_std_read_error() {
    let mut reader = FailingStdReader;
    let mut output = [0_u8; 1];

    let result = unsafe { Input::read_unchecked(&mut reader, &mut output, 0, 1) }
        .expect_err("read_unchecked should propagate std read errors");

    assert_eq!(ErrorKind::PermissionDenied, result.kind());
}
