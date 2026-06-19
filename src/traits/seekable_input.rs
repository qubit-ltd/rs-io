// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::{
    Input,
    Seekable,
};

/// Object-safe capability trait for inputs that can be read and repositioned
/// in the same item space.
///
/// `SeekableInput` exists to give the common [`Input`] + [`Seekable`]
/// combination a stable, named trait with a single item type. It is useful for
/// buffered inputs and other adapters that must adjust seek offsets by unread
/// items already pulled from the wrapped input.
///
/// [`Self::Item`] is the shared item type for both reading and seeking. It
/// always matches [`Input::Item`] and [`Seekable::Item`].
///
/// The trait adds no methods of its own. All operations come from the
/// supertraits, and every type implementing both [`Input`] and [`Seekable`]
/// with matching [`Input::Item`] and [`Seekable::Item`] automatically
/// implements `SeekableInput`.
pub trait SeekableInput:
    Input + Seekable<Item = <Self as Input>::Item>
{
    /// The item type shared by reading and seeking on this input.
    type Item;
}

impl<T> SeekableInput for T
where
    T: Input + Seekable<Item = <T as Input>::Item>,
{
    type Item = <T as Input>::Item;
}
