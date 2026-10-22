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
///
/// ```
/// use std::io;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use qubit_io::AsyncByteOutput;
///
/// fn write<S: AsyncByteOutput + ?Sized>(
///     stream: Pin<&mut S>, cx: &mut Context<'_>, input: & [u8],
/// ) -> Poll<io::Result<usize>> {
///     stream.poll_write(cx, input)
/// }
/// ```
///
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
