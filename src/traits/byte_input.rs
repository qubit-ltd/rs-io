// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Input;

/// A synchronous input whose items are bytes.
///
/// This trait adds no methods. Every [`Input<Item = u8>`](Input), including
/// unsized inputs and standard library readers, implements it automatically.
/// Implement [`Input`] to provide the operations; no separate implementation
/// of `ByteInput` is needed.
///
/// # Examples
///
/// ```
/// use qubit_io::ByteInput;
///
/// fn input() -> impl ByteInput {
///     std::io::Cursor::new(b"bytes")
/// }
///
/// let mut input: Box<dyn ByteInput> = Box::new(input());
/// let mut bytes = [0; 5];
/// input.read_exactly(&mut bytes)?;
/// assert_eq!(&bytes, b"bytes");
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// Inputs with non-byte items do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{ByteInput, Input};
///
/// fn accept(input: impl ByteInput) {}
///
/// fn reject(input: impl Input<Item = char>) {
///     accept(input);
/// }
/// ```
pub trait ByteInput: Input<Item = u8> {}

impl<T: Input<Item = u8> + ?Sized> ByteInput for T {}
