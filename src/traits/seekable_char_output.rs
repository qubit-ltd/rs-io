// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::CharOutput;
use crate::SeekableOutput;

/// A synchronous output that transfers and seeks in Unicode scalar values.
///
/// This trait adds no methods. Every [`SeekableOutput`] with both
/// `Item = char` and `Unit = char`, including unsized streams, implements it
/// automatically. Implement the underlying output and seek traits instead.
///
/// Positions count Unicode scalar values, not encoded bytes or grapheme
/// clusters. This trait does not encode or decode text, and does not promise
/// constant-time seeking.
///
/// # Examples
///
/// ```
/// use std::io::{Result, SeekFrom};
/// use qubit_io::SeekableCharOutput;
///
/// fn rewind(stream: &mut dyn SeekableCharOutput) -> Result<u64> {
///     stream.seek_to(SeekFrom::Start(0))
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{SeekableCharOutput, SeekableOutput};
///
/// fn accept(stream: impl SeekableCharOutput) {}
///
/// fn reject(stream: impl SeekableOutput<Item = u8, Unit = u8>) {
///     accept(stream);
/// }
/// ```
///
/// The seek unit must also match the transferred items:
///
/// ```compile_fail
/// use qubit_io::{Output, Seekable, SeekableCharOutput};
///
/// fn accept(stream: impl SeekableCharOutput) {}
///
/// fn reject(stream: impl Output<Item = char> + Seekable<Unit = u8>) {
///     accept(stream);
/// }
/// ```
pub trait SeekableCharOutput: CharOutput + SeekableOutput<Item = char, Unit = char> {}

impl<T: SeekableOutput<Item = char, Unit = char> + ?Sized> SeekableCharOutput for T {}
