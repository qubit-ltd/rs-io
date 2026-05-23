/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{Read, Write};

/// Object-safe capability trait for values that can be both read and written.
///
/// `ReadWrite` gives the common [`Read`] + [`Write`] combination a named trait
/// for APIs that work with duplex streams or mutable buffers through trait
/// objects. Typical use cases include protocol handlers, in-memory transport
/// tests, and adapters that should not care about the concrete stream type.
///
/// The trait adds no methods of its own. All operations come from the
/// standard-library supertraits, and every type implementing both [`Read`] and
/// [`Write`] automatically implements `ReadWrite`.
///
/// # Examples
///
/// ```rust
/// use qubit_io::ReadWrite;
///
/// fn send_ping(stream: &mut dyn ReadWrite) -> std::io::Result<()> {
///     stream.write_all(b"ping")
/// }
///
/// let mut cursor = std::io::Cursor::new(Vec::new());
/// send_ping(&mut cursor)?;
/// assert_eq!(cursor.into_inner(), b"ping");
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T where T: Read + Write + ?Sized {}
