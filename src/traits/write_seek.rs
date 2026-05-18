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
    Seek,
    Write,
};

/// Object-safe capability trait for values that can be written and repositioned.
///
/// `WriteSeek` gives the common [`Write`] + [`Seek`] combination a named trait
/// for APIs that write random-access output through trait objects. It is useful
/// for serializers, archive writers, and binary encoders that write placeholder
/// data first and later seek back to patch headers, offsets, or lengths.
///
/// The trait adds no methods of its own. All operations come from the
/// standard-library supertraits, and every type implementing both [`Write`] and
/// [`Seek`] automatically implements `WriteSeek`.
///
/// # Examples
///
/// ```rust
/// use qubit_io::WriteSeek;
/// use std::io::SeekFrom;
///
/// fn write_with_length(output: &mut dyn WriteSeek) -> std::io::Result<()> {
///     output.write_all(&[0])?;
///     output.write_all(b"data")?;
///     output.seek(SeekFrom::Start(0))?;
///     output.write_all(&[4])?;
///     Ok(())
/// }
///
/// let mut cursor = std::io::Cursor::new(Vec::new());
/// write_with_length(&mut cursor)?;
/// assert_eq!(cursor.into_inner(), b"\x04data");
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait WriteSeek: Write + Seek {}

impl<T> WriteSeek for T where T: Write + Seek + ?Sized {}
