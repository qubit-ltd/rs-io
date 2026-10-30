// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::AsyncOutput;

/// An asynchronous output whose items are bytes.
///
/// This trait adds no methods. Every [`AsyncOutput<Item = u8>`](AsyncOutput),
/// including unsized streams, implements it automatically. Implement
/// [`AsyncOutput`] to provide the operations; no separate implementation of
/// `AsyncByteOutput` is needed.
///
/// The inherited polling methods retain their pinning and readiness contracts;
/// implementors do not need to be `Unpin`.
///
/// # Examples
/// `ByteSink` is always ready and stores each written byte in memory:
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll, Waker};
/// use qubit_io::{AsyncByteOutput, AsyncOutput};
///
/// struct ByteSink(Vec<u8>);
/// impl AsyncOutput for ByteSink {
///     type Item = u8;
///     unsafe fn poll_write_unchecked(self: Pin<&mut Self>, _cx: &mut Context<'_>, input: &[u8], index: usize, count: usize) -> Poll<io::Result<usize>> {
///         let this = self.get_mut();
///         this.0.extend_from_slice(&input[index..index + count]);
///         Poll::Ready(Ok(count))
///     }
///     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
/// }
/// fn write<S: AsyncByteOutput + ?Sized>(stream: Pin<&mut S>, cx: &mut Context<'_>, input: &[u8]) -> Poll<io::Result<usize>> {
///     stream.poll_write(cx, input)
/// }
/// let mut sink = ByteSink(Vec::new());
/// let mut cx = Context::from_waker(Waker::noop());
/// assert!(matches!(write(Pin::new(&mut sink), &mut cx, b"QBIT"), Poll::Ready(Ok(4))));
/// assert_eq!(sink.0, b"QBIT");
/// assert!(matches!(Pin::new(&mut sink).poll_flush(&mut cx), Poll::Ready(Ok(()))));
/// ```
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{AsyncOutput, AsyncByteOutput};
///
/// fn accept(stream: impl AsyncByteOutput) {}
///
/// fn reject(stream: impl AsyncOutput<Item = char>) {
///     accept(stream);
/// }
/// ```
pub trait AsyncByteOutput: AsyncOutput<Item = u8> {}

impl<T: AsyncOutput<Item = u8> + ?Sized> AsyncByteOutput for T {}
