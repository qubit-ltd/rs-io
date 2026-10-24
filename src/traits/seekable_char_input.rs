// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::CharInput;
use crate::SeekableInput;

/// A synchronous input that transfers and seeks in Unicode scalar values.
///
/// This trait adds no methods. Every [`SeekableInput`] with both
/// `Item = char` and `Unit = char`, including unsized streams, implements it
/// automatically. Implement the underlying input and seek traits instead.
///
/// Positions count Unicode scalar values, not encoded bytes or grapheme
/// clusters. This trait does not encode or decode text, and does not promise
/// constant-time seeking.
///
/// # Examples
///
/// ```
/// use std::io::{Result, SeekFrom};
/// use qubit_io::SeekableCharInput;
///
/// fn rewind(stream: &mut dyn SeekableCharInput) -> Result<u64> {
///     stream.seek_to(SeekFrom::Start(0))
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{SeekableCharInput, SeekableInput};
///
/// fn accept(stream: impl SeekableCharInput) {}
///
/// fn reject(stream: impl SeekableInput<Item = u8, Unit = u8>) {
///     accept(stream);
/// }
/// ```
///
/// The seek unit must also match the transferred items:
///
/// ```compile_fail
/// use qubit_io::{Input, Seekable, SeekableCharInput};
///
/// fn accept(stream: impl SeekableCharInput) {}
///
/// fn reject(stream: impl Input<Item = char> + Seekable<Unit = u8>) {
///     accept(stream);
/// }
/// ```
pub trait SeekableCharInput: CharInput + SeekableInput<Item = char, Unit = char> {}

impl<T: SeekableInput<Item = char, Unit = char> + ?Sized> SeekableCharInput for T {}
