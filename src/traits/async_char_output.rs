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
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use qubit_io::AsyncCharOutput;
///
/// fn write<S: AsyncCharOutput + ?Sized>(
///     stream: Pin<&mut S>, cx: &mut Context<'_>, input: & [char],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_write(cx, input)
/// }
/// ```
///
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
