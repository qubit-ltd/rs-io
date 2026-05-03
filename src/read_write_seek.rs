/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    Read,
    Seek,
    Write,
};

/// Object-safe capability trait for values that can be read, written, and
/// repositioned.
///
/// `ReadWriteSeek` gives the common [`Read`] + [`Write`] + [`Seek`] combination
/// a named trait for APIs that need full mutable random-access I/O through a
/// trait object. Typical use cases include in-place file updates, editable
/// binary containers, test buffers, and components that should accept either a
/// file-like handle or an in-memory cursor.
///
/// The trait adds no methods of its own. All operations come from the
/// standard-library supertraits, and every type implementing [`Read`],
/// [`Write`], and [`Seek`] automatically implements `ReadWriteSeek`.
///
/// # Examples
///
/// ```rust
/// use qubit_io::ReadWriteSeek;
/// use std::io::SeekFrom;
///
/// fn rewrite_first_byte(io: &mut dyn ReadWriteSeek) -> std::io::Result<String> {
///     io.write_all(b"abc")?;
///     io.seek(SeekFrom::Start(0))?;
///     io.write_all(b"z")?;
///     io.seek(SeekFrom::Start(0))?;
///
///     let mut output = String::new();
///     io.read_to_string(&mut output)?;
///     Ok(output)
/// }
///
/// let mut cursor = std::io::Cursor::new(Vec::new());
/// assert_eq!(rewrite_first_byte(&mut cursor)?, "zbc");
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait ReadWriteSeek: Read + Write + Seek {}

impl<T> ReadWriteSeek for T where T: Read + Write + Seek {}
