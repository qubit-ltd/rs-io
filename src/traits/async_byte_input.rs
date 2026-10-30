// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::AsyncInput;

/// An asynchronous input whose items are bytes.
///
/// This trait adds no methods. Every [`AsyncInput<Item = u8>`](AsyncInput),
/// including unsized streams, implements it automatically. Implement
/// [`AsyncInput`] to provide the operations; no separate implementation of
/// `AsyncByteInput` is needed.
///
/// The inherited polling methods retain their pinning and readiness contracts;
/// implementors do not need to be `Unpin`.
///
/// # Examples
/// `ByteSlice` is always ready and reads from an in-memory byte slice:
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::Context;
/// use std::task::Poll;
/// use std::task::Waker;
///
/// use qubit_io::AsyncByteInput;
/// use qubit_io::AsyncInput;
///
/// struct ByteSlice<'a> {
///     items: &'a [u8],
///     position: usize,
/// }
///
/// impl AsyncInput for ByteSlice<'_> {
///     type Item = u8;
///
///     unsafe fn poll_read_unchecked(
///         self: Pin<&mut Self>,
///         _cx: &mut Context<'_>,
///         output: &mut [u8],
///         index: usize,
///         count: usize,
///     ) -> Poll<io::Result<usize>> {
///         let this = self.get_mut();
///         let n = count.min(this.items.len() - this.position);
///         output[index..index + n]
///             .copy_from_slice(&this.items[this.position..this.position + n]);
///         this.position += n;
///         Poll::Ready(Ok(n))
///     }
/// }
///
/// fn read<S: AsyncByteInput + ?Sized>(
///     stream: Pin<&mut S>,
///     cx: &mut Context<'_>,
///     output: &mut [u8],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_read(cx, output)
/// }
///
/// let mut stream = ByteSlice {
///     items: b"QBIT",
///     position: 0,
/// };
/// let mut cx = Context::from_waker(Waker::noop());
/// let mut output = [0; 4];
/// assert!(matches!(
///     read(Pin::new(&mut stream), &mut cx, &mut output),
///     Poll::Ready(Ok(4))
/// ));
/// assert_eq!(&output, b"QBIT");
/// assert!(matches!(
///     read(Pin::new(&mut stream), &mut cx, &mut output),
///     Poll::Ready(Ok(0))
/// ));
/// ```
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::AsyncByteInput;
/// use qubit_io::AsyncInput;
///
/// fn accept(stream: impl AsyncByteInput) {}
///
/// fn reject(stream: impl AsyncInput<Item = char>) {
///     accept(stream);
/// }
/// ```
pub trait AsyncByteInput: AsyncInput<Item = u8> {}

impl<T: AsyncInput<Item = u8> + ?Sized> AsyncByteInput for T {}
