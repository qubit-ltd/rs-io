// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Input;

/// A synchronous input whose items are Unicode scalar values.
///
/// This trait adds no methods. Every [`Input<Item = char>`](Input),
/// including unsized streams, implements it automatically. Implement [`Input`]
/// to provide the operations; no separate implementation of `CharInput` is
/// needed.
///
/// Character counts measure Unicode scalar values, not encoded bytes or
/// grapheme clusters. This trait does not encode or decode text.
///
/// # Examples
///
/// ```
/// use std::io;
///
/// use qubit_io::CharInput;
///
/// fn read_char(input: &mut impl CharInput) -> io::Result<char> {
///     let mut chars = ['\0'];
///     input.read_exactly(&mut chars)?;
///     Ok(chars[0])
/// }
/// ```
///
/// Streams with a different item type do not satisfy this bound:
/// ```compile_fail
/// use qubit_io::CharInput;
/// use qubit_io::Input;
///
/// fn accept(stream: impl CharInput) {}
///
/// fn reject(stream: impl Input<Item = u8>) {
///     accept(stream);
/// }
/// ```
pub trait CharInput: Input<Item = char> {
    // empty
}

impl<T: Input<Item = char> + ?Sized> CharInput for T {}
