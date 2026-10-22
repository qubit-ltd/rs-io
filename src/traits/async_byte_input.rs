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
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use qubit_io::AsyncByteInput;
///
/// fn read<S: AsyncByteInput + ?Sized>(
///     stream: Pin<&mut S>, cx: &mut Context<'_>, output: &mut [u8],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_read(cx, output)
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{AsyncInput, AsyncByteInput};
///
/// fn accept(stream: impl AsyncByteInput) {}
///
/// fn reject(stream: impl AsyncInput<Item = char>) {
///     accept(stream);
/// }
/// ```
pub trait AsyncByteInput: AsyncInput<Item = u8> {}

impl<T: AsyncInput<Item = u8> + ?Sized> AsyncByteInput for T {}
