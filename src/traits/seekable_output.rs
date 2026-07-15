// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::{Output, Seekable};

/// Object-safe capability trait for outputs that can be written and
/// repositioned in the same item space.
///
/// `SeekableOutput` exists to give the common [`Output`] + [`Seekable`]
/// combination a stable, named trait with a single item type. It is useful for
/// buffered outputs and other adapters that must coordinate pending items when
/// seeking the wrapped output.
///
/// [`Output::Item`] is the shared type for both writing and seeking. It
/// always matches [`Seekable::Unit`].
///
/// The trait adds no methods of its own. All operations come from the
/// supertraits, and every type implementing both [`Output`] and [`Seekable`]
/// with matching [`Output::Item`] and [`Seekable::Unit`] automatically
/// implements `SeekableOutput`.
///
/// # Examples
///
/// ```
/// use std::io::{
///     Cursor,
///     SeekFrom,
/// };
///
/// use qubit_io::SeekableOutput;
///
/// let output = Cursor::new(Vec::<u8>::new());
/// let mut output: Box<dyn SeekableOutput<Item = u8, Unit = u8>> =
///     Box::new(output);
///
/// assert_eq!(1, output.write(&[1_u8])?);
/// assert_eq!(0, output.seek_to(SeekFrom::Start(0))?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait SeekableOutput: Output + Seekable<Unit = <Self as Output>::Item> {}

impl<T> SeekableOutput for T where T: Output + Seekable<Unit = <T as Output>::Item> {}
