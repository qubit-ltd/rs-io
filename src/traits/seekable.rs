// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, Seek, SeekFrom};

/// Minimal seek interface measured in stream items.
///
/// Unlike [`Seek`], which measures positions and offsets in bytes,
/// `Seekable` measures them in units of [`Self::Unit`]. For byte streams,
/// set `Unit = u8`; offsets passed through [`SeekFrom`] then count units
/// rather than bytes.
///
/// The return value of [`Seekable::seek_to`] is the new absolute position from
/// the start of the stream, in items.
///
/// # Method name overlap
///
/// `Seekable::seek_to` has the same method name as [`Seek::seek`]. In generic
/// code where both traits are in scope for the same value, use fully qualified
/// syntax to choose item-oriented seeking or byte-oriented seeking explicitly:
///
/// ```
/// use std::io::{
///     Result,
///     Seek,
///     SeekFrom,
/// };
///
/// use qubit_io::Seekable;
///
/// fn seek_items<T>(stream: &mut T, position: SeekFrom) -> Result<u64>
/// where
///     T: Seekable + Seek,
/// {
///     <T as Seekable>::seek_to(stream, position)
/// }
///
/// fn seek_bytes<T>(stream: &mut T, position: SeekFrom) -> Result<u64>
/// where
///     T: Seekable + Seek,
/// {
///     Seek::seek(stream, position)
/// }
/// ```
///
/// # Coherence note
///
/// The blanket impl below maps [`std::io::Seek`] to `Unit = u8` for binary
/// compatibility. If a concrete type already implements `Seek`, it already has
/// an implicit `Seekable<Unit = u8>` impl from this blanket, so another
/// `Seekable` impl with the same `(Self, Unit)` pair would be a coherence
/// conflict.
///
/// For example, this is rejected by the compiler:
///
/// ```rust,compile_fail
/// use std::io::{Result, Seek, SeekFrom};
///
/// struct LegacyStream;
///
/// impl Seek for LegacyStream {
///     fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
///         Ok(0)
///     }
/// }
///
/// impl crate::Seekable for LegacyStream {
///     type Unit = u8;
///     fn seek_to(&mut self, _pos: SeekFrom) -> Result<u64> {
///         Ok(0)
///     }
/// }
/// ```
///
/// ```text
/// error[E0119]: conflicting implementations of trait `crate::Seekable`
/// for type `LegacyStream`
/// ```
///
/// The stable workaround is to keep byte-positioned seeking on the original
/// type and introduce a wrapper/newtype when another item interpretation is
/// needed: implement `Seekable` for the wrapper with a different `Unit`, and
/// keep `std::io::Seek`/byte semantics on the original type.
pub trait Seekable {
    /// The unit type used to measure seek positions and offsets.
    type Unit;

    /// Seeks to a position in the stream.
    ///
    /// # Parameters
    ///
    /// * `position` - Target position relative to the start, end, or current
    ///   logical position. All offsets are counted in units of [`Self::Unit`].
    ///
    /// # Returns
    ///
    /// The new absolute stream position, in items.
    ///
    /// # Errors
    ///
    /// Returns the seek error reported by the implementation.
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64>;
}

impl<S> Seekable for S
where
    S: Seek + ?Sized,
{
    type Unit = u8;

    /// Seeks a standard [`Seek`] value using byte offsets.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> Result<u64> {
        Seek::seek(self, position)
    }
}
