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
/// This example seeks by Unicode scalar value; its bounds are an example
/// policy:
///
/// ```
/// use std::io;
/// use std::io::SeekFrom;
/// use qubit_io::{Output, Seekable, SeekableCharOutput};
/// struct CharCursor { items: Vec<char>, position: usize }
/// impl Output for CharCursor {
///     type Item = char;
///     unsafe fn write_unchecked(&mut self, input: &[char], index: usize, count: usize) -> io::Result<usize> {
///         let n = count.min(self.items.len() - self.position);
///         self.items[self.position..self.position + n].copy_from_slice(&input[index..index + n]);
///         self.position += n;
///         Ok(n)
///     }
///     fn flush(&mut self) -> io::Result<()> { Ok(()) }
/// }
/// impl Seekable for CharCursor {
///     type Unit = char;
///     fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
///         let target = match position {
///             SeekFrom::Start(offset) => i128::from(offset),
///             SeekFrom::Current(offset) => self.position as i128 + i128::from(offset),
///             SeekFrom::End(offset) => self.items.len() as i128 + i128::from(offset),
///         };
///         if target < 0 || target > self.items.len() as i128 {
///             return Err(io::Error::new(io::ErrorKind::InvalidInput, "outside the example output"));
///         }
///         self.position = target as usize;
///         Ok(self.position as u64)
///     }
/// }
/// let mut cursor = CharCursor { items: vec!['你', '🦀'], position: 0 };
/// let stream: &mut dyn SeekableCharOutput = &mut cursor;
/// assert_eq!(stream.seek_to(SeekFrom::Start(1))?, 1);
/// stream.write_fully(&['好'])?;
/// assert_eq!(stream.seek_to(SeekFrom::Start(0))?, 0);
/// assert_eq!(cursor.items, ['你', '好']);
/// assert_eq!(cursor.position, 0);
/// # Ok::<(), io::Error>(())
/// ```
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
