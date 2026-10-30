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
/// Seeking to position 1 selects the second Unicode scalar, not a UTF-8 byte
/// offset.
///
/// # Examples
///
/// This memory stream checks seek bounds as an example policy; the trait does
/// not impose that restriction:
/// ```
/// use std::io;
/// use std::io::SeekFrom;
///
/// use qubit_io::Input;
/// use qubit_io::Seekable;
/// use qubit_io::SeekableCharInput;
///
/// struct CharCursor<'a> {
///     items: &'a [char],
///     position: usize,
/// }
///
/// impl Input for CharCursor<'_> {
///     type Item = char;
///
///     unsafe fn read_unchecked(
///         &mut self,
///         output: &mut [char],
///         index: usize,
///         count: usize,
///     ) -> io::Result<usize> {
///         let read = count.min(self.items.len() - self.position);
///         output[index..index + read]
///             .copy_from_slice(&self.items[self.position..self.position + read]);
///         self.position += read;
///         Ok(read)
///     }
/// }
///
/// impl Seekable for CharCursor<'_> {
///     type Unit = char;
///
///     fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
///         let target = match position {
///             SeekFrom::Start(offset) => i128::from(offset),
///             SeekFrom::Current(offset) => self.position as i128 + i128::from(offset),
///             SeekFrom::End(offset) => self.items.len() as i128 + i128::from(offset),
///         };
///         if target < 0 || target > self.items.len() as i128 {
///             return Err(io::Error::new(io::ErrorKind::InvalidInput, "outside the example input"));
///         }
///         self.position = target as usize;
///         Ok(self.position as u64)
///     }
/// }
///
/// fn rewind(stream: &mut dyn SeekableCharInput) -> io::Result<u64> {
///     stream.seek_to(SeekFrom::Start(0))
/// }
///
/// let mut stream = CharCursor { items: &['你', '🦀', 'a'], position: 0 };
/// assert_eq!(stream.seek_to(SeekFrom::Start(1)).unwrap(), 1);
/// let mut output = ['\0'; 1];
/// assert_eq!(stream.read(&mut output).unwrap(), 1);
/// assert_eq!(output, ['🦀']);
/// assert_eq!(rewind(&mut stream).unwrap(), 0);
/// assert_eq!(stream.read(&mut output).unwrap(), 1);
/// assert_eq!(output, ['你']);
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
