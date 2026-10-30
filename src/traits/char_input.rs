// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Input;

/// A synchronous input whose items are Unicode scalar values.
///
/// This trait adds no methods. Every [`Input<Item = char>`](Input),
/// including unsized streams, implements it automatically. Implement [`Input`]
/// to provide the operations; no separate implementation of `CharInput` is
/// needed.
///
/// Character counts measure Unicode scalar values, not encoded bytes or
/// grapheme clusters. This trait does not encode or decode text.
///
/// # Examples
/// A character input can be implemented directly over Unicode scalar values:
///
/// ```
/// use std::io;
/// use qubit_io::CharInput;
/// use qubit_io::Input;
///
/// struct CharSlice<'a> { items: &'a [char], position: usize }
/// impl Input for CharSlice<'_> {
///     type Item = char;
///     unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> io::Result<usize> {
///         let n = count.min(self.items.len() - self.position);
///         output[index..index + n].copy_from_slice(&self.items[self.position..self.position + n]);
///         self.position += n;
///         Ok(n)
///     }
/// }
/// fn read_char(input: &mut impl CharInput) -> io::Result<char> {
///     let mut chars = ['\0'];
///     input.read_exactly(&mut chars)?;
///     Ok(chars[0])
/// }
/// let mut input = CharSlice { items: &['你', '🦀'], position: 0 };
/// assert_eq!(read_char(&mut input)?, '你');
/// assert_eq!(read_char(&mut input)?, '🦀');
/// let mut empty = [];
/// assert_eq!(Input::read(&mut input, &mut empty)?, 0);
/// assert_eq!(input.position, 2);
/// let mut extra = ['\0'];
/// assert_eq!(Input::read(&mut input, &mut extra)?, 0);
/// # Ok::<(), io::Error>(())
/// ```
/// Streams with a different item type do not satisfy this bound:
/// ```compile_fail
/// use qubit_io::CharInput;
/// use qubit_io::Input;
///
/// fn accept(stream: impl CharInput) {}
///
/// fn reject(stream: impl Input<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait CharInput: Input<Item = char> {
    // empty
}

impl<T: Input<Item = char> + ?Sized> CharInput for T {}
