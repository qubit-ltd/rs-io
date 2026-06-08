// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{BufRead, Seek};

/// Object-safe capability trait for values that can be buffered-read and
/// repositioned.
///
/// `BufReadSeek` gives the common [`BufRead`] + [`Seek`] combination a named
/// trait for APIs that need buffered reads and random access through trait
/// objects. Typical use cases include line-oriented parsers over seekable files
/// and buffered readers that still need to jump between sections.
///
/// The trait adds no methods of its own. All operations come from the
/// standard-library supertraits, and every type implementing both [`BufRead`]
/// and [`Seek`] automatically implements `BufReadSeek`.
///
/// # Examples
///
/// ```rust
/// use qubit_io::BufReadSeek;
/// use std::io::{BufRead, BufReader, Cursor, SeekFrom};
///
/// fn read_after_prefix(input: &mut dyn BufReadSeek) -> std::io::Result<String> {
///     input.seek(SeekFrom::Start(4))?;
///
///     let mut line = String::new();
///     input.read_line(&mut line)?;
///     Ok(line)
/// }
///
/// let cursor = Cursor::new(b"abc\ndef".to_vec());
/// let mut reader = BufReader::new(cursor);
/// assert_eq!(read_after_prefix(&mut reader)?, "def");
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait BufReadSeek: BufRead + Seek {}

impl<T> BufReadSeek for T where T: BufRead + Seek + ?Sized {}
