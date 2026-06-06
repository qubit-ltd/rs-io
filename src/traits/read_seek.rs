// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    Read,
    Seek,
};

/// Object-safe capability trait for values that can be read and repositioned.
///
/// `ReadSeek` exists to give the common [`Read`] + [`Seek`] combination a
/// stable, named trait that can be used behind `dyn`. It is useful for APIs
/// that need to accept heterogeneous random-access inputs, such as files,
/// cursors, archive members, or custom readers, while still calling both
/// reading and seeking methods through the same trait object.
///
/// The trait adds no methods of its own. All operations come from the
/// standard-library supertraits, and every type implementing both [`Read`] and
/// [`Seek`] automatically implements `ReadSeek`.
///
/// # Examples
///
/// ```rust
/// use qubit_io::ReadSeek;
/// use std::io::SeekFrom;
///
/// fn read_from_start(input: &mut dyn ReadSeek) -> std::io::Result<Vec<u8>> {
///     input.seek(SeekFrom::Start(0))?;
///
///     let mut bytes = Vec::new();
///     input.read_to_end(&mut bytes)?;
///     Ok(bytes)
/// }
///
/// let mut cursor = std::io::Cursor::new(b"abc".to_vec());
/// assert_eq!(read_from_start(&mut cursor)?, b"abc");
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait ReadSeek: Read + Seek {}

impl<T> ReadSeek for T where T: Read + Seek + ?Sized {}
