// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::Buffer;

/// Describes the recovery status of items retained by a synchronous buffered
/// output.
///
/// `Pending` is safe to retry according to completed write results.
/// `Indeterminate` means an inner I/O operation panicked and may have produced
/// external effects that cannot be matched to the retained buffer.
///
/// # Examples
///
/// ```
/// use qubit_io::BufferedOutput;
/// use qubit_io::BufferedOutputRecovery;
/// use std::io::Cursor;
///
/// let mut output = BufferedOutput::with_capacity(Cursor::new(Vec::new()), 8);
/// output.write_fully(b"pending")?;
/// let (inner, recovery) = output.into_parts();
/// match recovery {
///     BufferedOutputRecovery::Pending(buffer) => assert_eq!(buffer.readable(), b"pending"),
///     BufferedOutputRecovery::Indeterminate(_) => unreachable!(),
/// }
/// drop(inner);
/// # Ok::<(), std::io::Error>(())
/// ```
#[must_use]
#[derive(Debug)]
pub enum BufferedOutputRecovery<T>
where
    T: Clone + Default,
{
    /// Items not reported as written by any normally returning inner call.
    ///
    /// The contained buffer remains in logical write order and may be retried,
    /// subject to any direct changes made through `inner_mut`.
    Pending(Buffer<T>),
    /// Items retained after an inner I/O operation panicked.
    ///
    /// The inner operation may have committed any prefix of these items, so
    /// automatically replaying the buffer could duplicate output. An empty
    /// buffer does not prove that the panicking operation had no effects.
    Indeterminate(Buffer<T>),
}
