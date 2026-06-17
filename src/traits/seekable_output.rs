// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::{
    Output,
    Seekable,
};

/// Object-safe capability trait for outputs that can be written and
/// repositioned in the same unit space.
///
/// `SeekableOutput` exists to give the common [`Output`] + [`Seekable`]
/// combination a stable, named trait with a single unit type. It is useful for
/// buffered outputs and other adapters that must flush pending units before
/// seeking the wrapped output.
///
/// [`Self::Item`] is the shared unit type for both writing and seeking. It
/// always matches [`Output::Item`] and [`Seekable::Item`].
///
/// The trait adds no methods of its own. All operations come from the
/// supertraits, and every type implementing both [`Output`] and [`Seekable`]
/// with matching [`Output::Item`] and [`Seekable::Item`] automatically
/// implements `SeekableOutput`.
pub trait SeekableOutput:
    Output + Seekable<Item = <Self as Output>::Item>
{
    /// The unit type shared by writing and seeking on this output.
    type Item;
}

impl<T> SeekableOutput for T
where
    T: Output + Seekable<Item = <T as Output>::Item>,
{
    type Item = <T as Output>::Item;
}
