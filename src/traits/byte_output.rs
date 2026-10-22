// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Output;

/// A synchronous output whose items are bytes.
///
/// This trait adds no methods. Every [`Output<Item = u8>`](Output), including
/// unsized outputs and standard library writers, implements it automatically.
/// Implement [`Output`] to provide the operations; no separate implementation
/// of `ByteOutput` is needed.
///
/// # Examples
///
/// ```
/// use qubit_io::ByteOutput;
///
/// fn output(bytes: &mut Vec<u8>) -> impl ByteOutput + '_ {
///     bytes
/// }
///
/// let mut bytes = Vec::new();
/// {
///     let mut output: Box<dyn ByteOutput + '_> = Box::new(output(&mut bytes));
///     output.write_fully(b"bytes")?;
/// }
/// assert_eq!(&bytes, b"bytes");
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// Outputs with non-byte items do not satisfy this bound:
///
/// ```compile_fail
/// use qubit_io::{ByteOutput, Output};
///
/// fn accept(output: impl ByteOutput) {}
///
/// fn reject(output: impl Output<Item = char>) {
///     accept(output);
/// }
/// ```
pub trait ByteOutput: Output<Item = u8> {}

impl<T: Output<Item = u8> + ?Sized> ByteOutput for T {}
