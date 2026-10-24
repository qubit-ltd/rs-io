// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::ByteOutput;
use crate::SeekableOutput;

/// A synchronous output that transfers and seeks in bytes.
///
/// This trait adds no methods. Every [`SeekableOutput`] with both
/// `Item = u8` and `Unit = u8`, including unsized streams, implements it
/// automatically. Implement the underlying output and seek traits instead.
///
/// # Examples
///
/// ```
/// use std::io::{Result, SeekFrom};
/// use qubit_io::SeekableByteOutput;
///
/// fn rewind(stream: &mut dyn SeekableByteOutput) -> Result<u64> {
///     stream.seek_to(SeekFrom::Start(0))
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{SeekableByteOutput, SeekableOutput};
///
/// fn accept(stream: impl SeekableByteOutput) {}
///
/// fn reject(stream: impl SeekableOutput<Item = char, Unit = char>) {
///     accept(stream);
/// }
/// ```
///
/// The seek unit must also match the transferred items:
///
/// ```compile_fail
/// use qubit_io::{Output, Seekable, SeekableByteOutput};
///
/// fn accept(stream: impl SeekableByteOutput) {}
///
/// fn reject(stream: impl Output<Item = u8> + Seekable<Unit = char>) {
///     accept(stream);
/// }
/// ```
pub trait SeekableByteOutput: ByteOutput + SeekableOutput<Item = u8, Unit = u8> {}

impl<T: SeekableOutput<Item = u8, Unit = u8> + ?Sized> SeekableByteOutput for T {}
