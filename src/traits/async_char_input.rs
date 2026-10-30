// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::AsyncInput;

/// An asynchronous input whose items are Unicode scalar values.
///
/// This trait adds no methods. Every [`AsyncInput<Item = char>`](AsyncInput),
/// including unsized streams, implements it automatically. Implement
/// [`AsyncInput`] to provide the operations; no separate implementation of
/// `AsyncCharInput` is needed.
///
/// Character counts measure Unicode scalar values, not encoded bytes or
/// grapheme clusters. This trait does not encode or decode text.
///
/// The inherited polling methods retain their pinning and readiness contracts;
/// implementors do not need to be `Unpin`.
///
/// # Examples
///
/// `CharSlice` is always ready, so it does not need to register a waker:
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// use qubit_io::AsyncCharInput;
/// use qubit_io::AsyncInput;
///
/// struct CharSlice<'a> {
///     items: &'a [char],
///     position: usize,
/// }
///
/// impl AsyncInput for CharSlice<'_> {
///     type Item = char;
///
///     unsafe fn poll_read_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         output: &mut [char],
///         index: usize,
///         count: usize,
///     ) -> Poll<io::Result<usize>> {
///         let this = self.get_mut();
///         let read = count.min(this.items.len() - this.position);
///         output[index..index + read]
///             .copy_from_slice(&this.items[this.position..this.position + read]);
///         this.position += read;
///         Poll::Ready(Ok(read))
///     }
/// }
///
/// fn read<S: AsyncCharInput + ?Sized>(
///     stream: Pin<&mut S>,
///     cx: &mut Context<'_>,
///     output: &mut [char],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_read(cx, output)
/// }
///
/// let mut stream = CharSlice { items: &['你', '🦀'], position: 0 };
/// let mut cx = Context::from_waker(Waker::noop());
/// let mut output = ['\0'; 2];
/// assert!(matches!(read(Pin::new(&mut stream), &mut cx, &mut output), Poll::Ready(Ok(2))));
/// assert_eq!(output, ['你', '🦀']);
/// assert!(matches!(read(Pin::new(&mut stream), &mut cx, &mut output), Poll::Ready(Ok(0))));
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{AsyncInput, AsyncCharInput};
///
/// fn accept(stream: impl AsyncCharInput) {}
///
/// fn reject(stream: impl AsyncInput<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait AsyncCharInput: AsyncInput<Item = char> {}

impl<T: AsyncInput<Item = char> + ?Sized> AsyncCharInput for T {}
