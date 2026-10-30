// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Tests for the standard [`Write`](std::io::Write) bridge to [`Output`].

use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::io::IoSlice;
use std::io::Write;
#[cfg(debug_assertions)]
use std::panic::AssertUnwindSafe;
#[cfg(debug_assertions)]
use std::panic::catch_unwind;

use qubit_io::Output;

#[derive(Default)]
struct VectorStdWriter(Vec<u8>);

impl Write for VectorStdWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        panic!("scalar write called")
    }
    fn write_vectored(&mut self, buffers: &[IoSlice<'_>]) -> std::io::Result<usize> {
        let before = self.0.len();
        for buffer in buffers {
            self.0.extend_from_slice(buffer);
        }
        Ok(self.0.len() - before)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_std_writer_keeps_native_override() {
    let mut writer = VectorStdWriter::default();
    assert_eq!(Output::write_vectored(&mut writer, &[b"ab", b"cd"]).unwrap(), 4);
    assert_eq!(writer.0, b"abcd");
}

struct EdgeStdWriter {
    reply: Result<usize, ErrorKind>,
    bytes: Vec<u8>,
    calls: usize,
}

impl Write for EdgeStdWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        panic!("scalar write called")
    }

    fn write_vectored(&mut self, buffers: &[IoSlice<'_>]) -> std::io::Result<usize> {
        self.calls += 1;
        let count = self.reply.map_err(Error::from)?;
        let mut remaining = count;
        for buffer in buffers {
            let copied = remaining.min(buffer.len());
            self.bytes.extend_from_slice(&buffer[..copied]);
            remaining -= copied;
            if remaining == 0 {
                break;
            }
        }
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_std_writer_preserves_empty_short_error_and_count_contracts() {
    let mut writer = EdgeStdWriter {
        reply: Ok(0),
        bytes: Vec::new(),
        calls: 0,
    };
    assert_eq!(Output::write_vectored(&mut writer, &[]).unwrap(), 0);
    assert_eq!(Output::write_vectored(&mut writer, &[b"", b""]).unwrap(), 0);
    assert_eq!(writer.calls, 0);

    let mut writer = EdgeStdWriter {
        reply: Ok(1),
        bytes: Vec::new(),
        calls: 0,
    };
    assert_eq!(Output::write_vectored(&mut writer, &[b"ab", b"cd"]).unwrap(), 1);
    assert_eq!(writer.bytes, b"a");
    assert_eq!(writer.calls, 1);

    for kind in [ErrorKind::Interrupted, ErrorKind::PermissionDenied] {
        let mut writer = EdgeStdWriter {
            reply: Err(kind),
            bytes: Vec::new(),
            calls: 0,
        };
        assert_eq!(Output::write_vectored(&mut writer, &[b"a"]).unwrap_err().kind(), kind);
        assert_eq!(writer.calls, 1);
    }

    let mut writer = EdgeStdWriter {
        reply: Ok(5),
        bytes: Vec::new(),
        calls: 0,
    };
    assert_eq!(
        Output::write_vectored(&mut writer, &[b"ab", b"cd"]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(writer.calls, 1);
}

struct OverreportingStdWriter;

struct FailingStdWriter;

struct FailingFlushStdWriter;

impl Write for FailingStdWriter {
    /// Returns a deterministic standard writer failure.
    fn write(&mut self, _input: &[u8]) -> std::io::Result<usize> {
        Err(Error::new(ErrorKind::BrokenPipe, "write failed"))
    }

    /// Completes flushing because the test writer buffers no data.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for FailingFlushStdWriter {
    /// Returns deterministic write success so tests can isolate flush errors.
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        Ok(input.len())
    }

    /// Fails every flush attempt.
    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::new(ErrorKind::BrokenPipe, "flush failed"))
    }
}

impl Write for OverreportingStdWriter {
    /// Deliberately violates the standard writer count contract.
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        Ok(input.len() + 1)
    }

    /// Completes flushing because the test writer buffers no data.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Verifies normal, indexed, and flush operations through the standard writer
/// bridge.
#[test]
fn test_write_blanket_impl_exposes_output_methods() {
    let mut cursor = Cursor::new(Vec::new());

    // SAFETY: b"bc" is a valid source range inside b"abc".
    let written =
        unsafe { Output::write_unchecked(&mut cursor, b"abc", 1, 2).expect("write_unchecked should succeed") };
    assert_eq!(2, written);
    assert_eq!(b"bc", cursor.into_inner().as_slice());

    let mut cursor = Cursor::new(Vec::new());
    let written = Output::write(&mut cursor, b"xy").expect("Output::write should delegate to write_unchecked");
    assert_eq!(2, written);
    assert_eq!(b"xy", cursor.into_inner().as_slice());

    let mut cursor = Cursor::new(Vec::new());
    Write::write_all(&mut cursor, b"z").expect("seed bytes for flush");
    Output::flush(&mut cursor).expect("flush should succeed");
}

/// Verifies that the standard writer bridge rejects impossible write counts.
#[test]
fn test_write_blanket_impl_rejects_overreported_count() {
    let mut writer = OverreportingStdWriter;

    let error = Output::write(&mut writer, b"abc").expect_err("blanket output should validate the Write count");

    assert_eq!(ErrorKind::InvalidData, error.kind());
}

/// Verifies that the standard writer bridge preserves writer errors.
#[test]
fn test_write_blanket_impl_propagates_std_write_error() {
    let mut writer = FailingStdWriter;

    let error = Output::write(&mut writer, b"abc").expect_err("blanket output should propagate Write errors");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
}

#[cfg(debug_assertions)]
#[test]
fn test_write_blanket_impl_write_unchecked_panics_on_invalid_range() {
    let mut writer = Cursor::new(Vec::new());

    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the range is intentionally invalid; this verifies
        // debug-assert behavior.
        let _ = unsafe { Output::write_unchecked(&mut writer, b"ab", 1, 2).expect("should panic") };
    }));

    assert!(result.is_err(), "write_unchecked should panic on invalid range");
}

#[cfg(debug_assertions)]
#[test]
fn test_write_blanket_impl_write_unchecked_panics_on_index_plus_count_overflow() {
    let mut writer = Cursor::new(Vec::new());

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = unsafe { Output::write_unchecked(&mut writer, b"ab", usize::MAX, 2).expect("should panic") };
    }));

    assert!(
        result.is_err(),
        "write_unchecked should panic when index + count overflows"
    );
}

#[test]
fn test_write_blanket_impl_write_unchecked_propagates_std_write_error() {
    let mut writer = FailingStdWriter;

    let result = unsafe { Output::write_unchecked(&mut writer, b"abc", 0, 1) }
        .expect_err("write_unchecked should propagate std write errors");

    assert_eq!(ErrorKind::BrokenPipe, result.kind());
}

#[test]
fn test_write_blanket_impl_flush_propagates_std_flush_error() {
    let mut writer = FailingFlushStdWriter;

    let error = Output::flush(&mut writer).expect_err("Output::flush should propagate std flush errors");

    assert_eq!(ErrorKind::BrokenPipe, error.kind());
}
