// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::{Input, Seekable};

/// Object-safe capability trait for inputs that can be read and repositioned
/// in the same item space.
///
/// `SeekableInput` exists to give the common [`Input`] + [`Seekable`]
/// combination a stable, named trait with a single item type. It is useful for
/// buffered inputs and other adapters that must adjust seek offsets by unread
/// items already pulled from the wrapped input.
///
/// [`Input::Item`] is the shared type for both reading and seeking. It always
/// matches [`Seekable::Unit`].
///
/// The trait adds no methods of its own. All operations come from the
/// supertraits, and every type implementing both [`Input`] and [`Seekable`]
/// with matching [`Input::Item`] and [`Seekable::Unit`] automatically
/// implements `SeekableInput`.
///
/// # Examples
///
/// ```
/// use std::io::{
///     Cursor,
///     SeekFrom,
/// };
///
/// use qubit_io::SeekableInput;
///
/// let input = Cursor::new(vec![1_u8, 2, 3]);
/// let mut input: Box<dyn SeekableInput<Item = u8, Unit = u8>> =
///     Box::new(input);
/// let mut byte = [0_u8; 1];
///
/// assert_eq!(1, input.read(&mut byte)?);
/// assert_eq!([1], byte);
/// assert_eq!(0, input.seek_to(SeekFrom::Start(0))?);
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait SeekableInput: Input + Seekable<Unit = <Self as Input>::Item> {}

impl<T> SeekableInput for T where T: Input + Seekable<Unit = <T as Input>::Item> {}
