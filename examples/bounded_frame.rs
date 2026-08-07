// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Demonstrates a bounded length-prefixed frame decoder.
//!
//! The decoder treats the length prefix as untrusted input: it validates the
//! declared size before allocating and uses checked [`Input`] helpers so
//! truncated headers and payloads are reported as errors.

use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::io::{self};

use qubit_io::Input;

/// Largest payload accepted by this example protocol.
const MAX_FRAME_LEN: usize = 64 * 1024;

/// Reads one four-byte big-endian length-prefixed frame.
///
/// # Parameters
///
/// - `input`: Byte input supplying the header and payload.
///
/// # Returns
///
/// Returns the frame payload, including an empty payload when the declared
/// length is zero.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] when the declared payload exceeds
/// [`MAX_FRAME_LEN`], and [`io::ErrorKind::UnexpectedEof`] when either the
/// header or payload is truncated.
fn read_frame<I>(input: &mut I) -> io::Result<Vec<u8>>
where
    I: Input<Item = u8>,
{
    let mut header = [0_u8; 4];
    input.read_exactly(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_LEN {
        return Err(Error::new(ErrorKind::InvalidData, "frame is too large"));
    }

    let mut payload = vec![0_u8; length];
    input.read_exactly(&mut payload)?;
    Ok(payload)
}

/// Runs the frame decoder against representative protocol inputs.
///
/// # Errors
///
/// Returns an error when an expected frame result or error category does not
/// match the decoder contract.
fn main() -> io::Result<()> {
    let mut frame = Cursor::new([0, 0, 0, 3, b'f', b'o', b'o']);
    assert_eq!(b"foo", read_frame(&mut frame)?.as_slice());

    let mut empty = Cursor::new([0, 0, 0, 0]);
    assert!(read_frame(&mut empty)?.is_empty());

    let mut oversized = Cursor::new(
        u32::try_from(MAX_FRAME_LEN + 1)
            .expect("frame limit fits in a u32")
            .to_be_bytes(),
    );
    let oversized_error = read_frame(&mut oversized)
        .expect_err("oversized frame must be rejected before allocation");
    assert_eq!(ErrorKind::InvalidData, oversized_error.kind());

    let mut truncated = Cursor::new([0, 0, 0, 2, b'x']);
    let truncated_error =
        read_frame(&mut truncated).expect_err("truncated payload must fail");
    assert_eq!(ErrorKind::UnexpectedEof, truncated_error.kind());
    Ok(())
}
