// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Output;

/// A synchronous output whose items are Unicode scalar values.
///
/// This trait adds no methods. Every [`Output<Item = char>`](Output),
/// including unsized streams, implements it automatically. Implement [`Output`]
/// to provide the operations; no separate implementation of `CharOutput` is
/// needed.
///
/// Character counts measure Unicode scalar values, not encoded bytes or
/// grapheme clusters. This trait does not encode or decode text.
///
/// # Examples
///
/// ```
/// use std::io;
/// use qubit_io::CharOutput;
///
/// fn write_char(output: &mut impl CharOutput, value: char) -> io::Result<()> {
///     output.write_fully(&[value])
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{Output, CharOutput};
///
/// fn accept(stream: impl CharOutput) {}
///
/// fn reject(stream: impl Output<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait CharOutput: Output<Item = char> {}

impl<T: Output<Item = char> + ?Sized> CharOutput for T {}
