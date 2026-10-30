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
///
/// use qubit_io::CharOutput;
/// use qubit_io::Output;
///
/// fn write_char(output: &mut impl CharOutput, value: char) -> io::Result<()> {
///     output.write_fully(&[value])
/// }
///
/// #[derive(Default)]
/// struct CharBuffer(Vec<char>);
///
/// impl Output for CharBuffer {
///     type Item = char;
///
///     unsafe fn write_unchecked(
///         &mut self,
///         input: &[char],
///         index: usize,
///         count: usize,
///     ) -> io::Result<usize> {
///         self.0.extend_from_slice(&input[index..index + count]);
///         Ok(count)
///     }
///
///     fn flush(&mut self) -> io::Result<()> {
///         Ok(())
///     }
/// }
///
/// let mut output = CharBuffer::default();
/// write_char(&mut output, '🦀')?;
/// assert_eq!(output.0, ['🦀']);
/// # Ok::<(), io::Error>(())
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::CharOutput;
/// use qubit_io::Output;
///
/// fn accept(stream: impl CharOutput) {}
///
/// fn reject(stream: impl Output<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait CharOutput: Output<Item = char> {}

impl<T: Output<Item = char> + ?Sized> CharOutput for T {}
