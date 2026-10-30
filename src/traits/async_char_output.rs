// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::AsyncOutput;

/// An asynchronous output whose items are Unicode scalar values.
///
/// This trait adds no methods. Every [`AsyncOutput<Item = char>`](AsyncOutput),
/// including unsized streams, implements it automatically. Implement
/// [`AsyncOutput`] to provide the operations; no separate implementation of
/// `AsyncCharOutput` is needed.
///
/// Character counts measure Unicode scalar values, not encoded bytes or
/// grapheme clusters. This trait does not encode or decode text.
///
/// The inherited polling methods retain their pinning and readiness contracts;
/// implementors do not need to be `Unpin`.
///
/// # Examples
/// `CharSink` is always ready and stores Unicode scalar values in memory:
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll, Waker};
/// use qubit_io::{AsyncCharOutput, AsyncOutput};
///
/// struct CharSink(Vec<char>);
/// impl AsyncOutput for CharSink {
///     type Item = char;
///     unsafe fn poll_write_unchecked(self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[char], index: usize, count: usize) -> Poll<io::Result<usize>> {
///         let this = self.get_mut();
///         this.0.extend_from_slice(&input[index..index + count]);
///         Poll::Ready(Ok(count))
///     }
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
/// }
/// fn write<S: AsyncCharOutput + ?Sized>(stream: Pin<&mut S>, cx: &mut Context<'_>, input: &[char]) -> Poll<io::Result<usize>> {
///     stream.poll_write(cx, input)
/// }
/// let mut sink = CharSink(Vec::new());
/// let mut cx = Context::from_waker(Waker::noop());
/// assert!(matches!(write(Pin::new(&mut sink), &mut cx, &['你', '🦀']), Poll::Ready(Ok(2))));
/// assert_eq!(sink.0, ['你', '🦀']);
/// assert!(matches!(Pin::new(&mut sink).poll_flush(&mut cx), Poll::Ready(Ok(()))));
/// ```
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{AsyncOutput, AsyncCharOutput};
///
/// fn accept(stream: impl AsyncCharOutput) {}
///
/// fn reject(stream: impl AsyncOutput<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait AsyncCharOutput: AsyncOutput<Item = char> {}

impl<T: AsyncOutput<Item = char> + ?Sized> AsyncCharOutput for T {}
