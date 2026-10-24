// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::ByteInput;
use crate::SeekableInput;

/// A synchronous input that transfers and seeks in bytes.
///
/// This trait adds no methods. Every [`SeekableInput`] with both
/// `Item = u8` and `Unit = u8`, including unsized streams, implements it
/// automatically. Implement the underlying input and seek traits instead.
///
/// # Examples
///
/// ```
/// use std::io::{Result, SeekFrom};
/// use qubit_io::SeekableByteInput;
///
/// fn rewind(stream: &mut dyn SeekableByteInput) -> Result<u64> {
///     stream.seek_to(SeekFrom::Start(0))
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{SeekableByteInput, SeekableInput};
///
/// fn accept(stream: impl SeekableByteInput) {}
///
/// fn reject(stream: impl SeekableInput<Item = char, Unit = char>) {
///     accept(stream);
/// }
/// ```
///
/// The seek unit must also match the transferred items:
///
/// ```compile_fail
/// use qubit_io::{Input, Seekable, SeekableByteInput};
///
/// fn accept(stream: impl SeekableByteInput) {}
///
/// fn reject(stream: impl Input<Item = u8> + Seekable<Unit = char>) {
///     accept(stream);
/// }
/// ```
pub trait SeekableByteInput: ByteInput + SeekableInput<Item = u8, Unit = u8> {}

impl<T: SeekableInput<Item = u8, Unit = u8> + ?Sized> SeekableByteInput for T {}
