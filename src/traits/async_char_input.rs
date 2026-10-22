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
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use qubit_io::AsyncCharInput;
///
/// fn read<S: AsyncCharInput + ?Sized>(
///     stream: Pin<&mut S>, cx: &mut Context<'_>, output: &mut [char],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_read(cx, output)
/// }
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
