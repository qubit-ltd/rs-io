// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Low-level unchecked slice helpers in a dedicated namespace.
//!
//! These helpers avoid bound checks and are intended for call sites that
//! already validate bounds in their own protocol.

use core::mem;
use std::convert::Infallible;
use std::io::{
    Error,
    ErrorKind,
    Result,
};

/// Namespace for low-level slice operations without bound checks.
///
/// All functions are unsafe and assume the caller has already validated their
/// preconditions. Safety requirements in each method are explicit.
pub struct UncheckedSlice {
    /// Prevents construction of this namespace type.
    _private: Infallible,
}

impl UncheckedSlice {
    /// Returns the exclusive end index of a checked slice range.
    ///
    /// # Parameters
    ///
    /// - `len`: Slice length.
    /// - `start`: Start index in the slice.
    /// - `count`: Number of requested items after `start`.
    ///
    /// # Returns
    ///
    /// `Some(end)` if `start + count <= len` and no overflow occurs, or
    /// `None` when the requested range does not fit inside the slice.
    #[inline]
    pub const fn range_end(
        len: usize,
        start: usize,
        count: usize,
    ) -> Option<usize> {
        match start.checked_add(count) {
            Some(end) if len >= end => Some(end),
            _ => None,
        }
    }

    /// Returns whether a slice has at least `count` readable/writable items
    /// from `start`.
    ///
    /// # Parameters
    ///
    /// - `len`: Slice length.
    /// - `start`: Start index in the slice.
    /// - `count`: Number of requested items after `start`.
    ///
    /// # Returns
    ///
    /// `true` if `start + count <= len` and no overflow occurs.
    #[must_use]
    #[inline(always)]
    pub const fn range_fits(len: usize, start: usize, count: usize) -> bool {
        Self::range_end(len, start, count).is_some()
    }

    /// Returns the exclusive end index of a checked slice range as an I/O
    /// result.
    ///
    /// # Parameters
    ///
    /// - `len`: Slice length.
    /// - `start`: Start index in the slice.
    /// - `count`: Number of requested items after `start`.
    /// - `message`: Error message used when the requested range is invalid.
    ///
    /// # Returns
    ///
    /// Returns the exclusive end index when the range fits inside the slice.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] with `message` when
    /// `start + count` overflows or exceeds `len`.
    #[inline]
    pub fn checked_range_end(
        len: usize,
        start: usize,
        count: usize,
        message: &'static str,
    ) -> Result<usize> {
        Self::range_end(len, start, count)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, message))
    }

    /// Reads one value from an unchecked slice index.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Copyable element type read from the slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    /// - `index`: Start index that must be valid for reading one item.
    ///
    /// # Returns
    ///
    /// A copy of the value stored at `index`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index < input.len()`.
    #[must_use]
    #[inline(always)]
    pub unsafe fn read<T: Copy>(input: &[T], index: usize) -> T {
        // SAFETY: The caller guarantees that `index` is in-bounds.
        unsafe { *input.as_ptr().add(index) }
    }

    /// Writes one value to an unchecked mutable slice index.
    ///
    /// This replaces the existing initialized element at `index`. The previous
    /// value is dropped before `value` is moved into the slot.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Element type stored by the slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    /// - `index`: Start index that must be valid for writing one item.
    /// - `value`: Value to write.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index < output.len()`.
    #[inline(always)]
    pub unsafe fn write<T>(output: &mut [T], index: usize, value: T) {
        // SAFETY: The caller guarantees that `index` is in-bounds.
        unsafe {
            *output.as_mut_ptr().add(index) = value;
        }
    }

    /// Returns an immutable reference to one value at an unchecked slice index.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Element type stored by the slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    /// - `index`: Start index that must be valid for reading one item.
    ///
    /// # Returns
    ///
    /// A shared reference to the value at `index`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index < input.len()`.
    #[must_use]
    #[inline(always)]
    pub unsafe fn get<T>(input: &[T], index: usize) -> &T {
        // SAFETY: The caller guarantees that `index` is in-bounds.
        unsafe { &*input.as_ptr().add(index) }
    }

    /// Returns a mutable reference to one value at an unchecked mutable slice
    /// index.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Element type stored by the slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    /// - `index`: Start index that must be valid for writing one item.
    ///
    /// # Returns
    ///
    /// An exclusive reference to the value at `index`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index < output.len()`.
    #[must_use]
    #[inline(always)]
    pub unsafe fn get_mut<T>(output: &mut [T], index: usize) -> &mut T {
        // SAFETY: The caller guarantees that `index` is in-bounds.
        unsafe { &mut *output.as_mut_ptr().add(index) }
    }

    /// Returns an immutable subslice at an unchecked offset and length.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Element type stored by the slice.
    ///
    /// # Parameters
    ///
    /// - `input`: Source slice.
    /// - `start`: Start index in `input`.
    /// - `count`: Number of items in the returned subslice.
    ///
    /// # Returns
    ///
    /// The shared subslice spanning the requested range.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested range does not fit.
    /// Callers do not need to repeat this range assertion.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `start + count <= input.len()` and that
    /// the addition does not overflow.
    #[must_use]
    #[inline(always)]
    pub unsafe fn subslice<T>(input: &[T], start: usize, count: usize) -> &[T] {
        debug_assert!(
            Self::range_fits(input.len(), start, count),
            "subslice range exceeds input buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside `input`.
        unsafe { core::slice::from_raw_parts(input.as_ptr().add(start), count) }
    }

    /// Returns a mutable subslice at an unchecked offset and length.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Element type stored by the slice.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination slice.
    /// - `start`: Start index in `output`.
    /// - `count`: Number of items in the returned subslice.
    ///
    /// # Returns
    ///
    /// The exclusive subslice spanning the requested range.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested range does not fit.
    /// Callers do not need to repeat this range assertion.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `start + count <= output.len()` and that
    /// the addition does not overflow.
    #[must_use]
    #[inline(always)]
    pub unsafe fn subslice_mut<T>(
        output: &mut [T],
        start: usize,
        count: usize,
    ) -> &mut [T] {
        debug_assert!(
            Self::range_fits(output.len(), start, count),
            "subslice range exceeds output buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `output`.
        unsafe {
            core::slice::from_raw_parts_mut(
                output.as_mut_ptr().add(start),
                count,
            )
        }
    }

    /// Copies `count` values between unchecked slice offsets.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Copyable element type stored by both slices.
    ///
    /// # Parameters
    ///
    /// - `source`: Source slice.
    /// - `source_index`: Source offset, must be valid for `count` items.
    /// - `destination`: Destination slice.
    /// - `destination_index`: Destination offset, must be valid for `count`
    ///   items.
    /// - `count`: Number of items to copy.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if either requested range does not fit.
    /// Callers do not need to repeat these range assertions.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that both source and destination ranges are
    /// valid for `count` elements, the copy does not overflow pointer
    /// arithmetic, and the two memory regions do not overlap.
    #[inline(always)]
    pub unsafe fn copy_nonoverlapping<T: Copy>(
        source: &[T],
        source_index: usize,
        destination: &mut [T],
        destination_index: usize,
        count: usize,
    ) {
        debug_assert!(
            Self::range_fits(source.len(), source_index, count),
            "unchecked source range exceeds source buffer"
        );
        debug_assert!(
            Self::range_fits(destination.len(), destination_index, count),
            "unchecked destination range exceeds destination buffer"
        );
        // SAFETY: The caller guarantees both ranges are valid and
        // non-overlapping.
        unsafe {
            let src = source.as_ptr().add(source_index);
            let dst = destination.as_mut_ptr().add(destination_index);
            core::ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    /// Copies `count` values between unchecked offsets in one buffer.
    ///
    /// Overlapping source and destination ranges are supported.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Copyable element type stored by the buffer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Buffer containing both ranges.
    /// - `source_index`: Source offset, must be valid for `count` items.
    /// - `destination_index`: Destination offset, must be valid for `count`
    ///   items.
    /// - `count`: Number of values to copy.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if either requested range does not fit.
    /// Callers do not need to repeat these range assertions.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that both ranges lie within `buffer` and that
    /// `source_index + count` and `destination_index + count` do not overflow
    /// `usize`.
    #[inline(always)]
    pub unsafe fn copy_within<T: Copy>(
        buffer: &mut [T],
        source_index: usize,
        destination_index: usize,
        count: usize,
    ) {
        debug_assert!(
            Self::range_fits(buffer.len(), source_index, count),
            "unchecked source range exceeds buffer"
        );
        debug_assert!(
            Self::range_fits(buffer.len(), destination_index, count),
            "unchecked destination range exceeds buffer"
        );
        // SAFETY: The caller guarantees both ranges are valid; `copy` supports
        // overlapping regions within the same allocation.
        unsafe {
            let base = buffer.as_mut_ptr();
            let source = base.add(source_index);
            let destination = base.add(destination_index);
            core::ptr::copy(source, destination, count);
        }
    }

    /// Reads one value from an unchecked unaligned byte slice offset.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Copyable value type represented by the source bytes.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte buffer.
    /// - `index`: Byte offset in `input`.
    ///
    /// # Returns
    ///
    /// The value reconstructed from the bytes at `index`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested byte range does not fit.
    /// Callers do not need to repeat this range assertion.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + size_of::<T>()` is a
    /// valid readable range inside `input` and that the addition does not
    /// overflow. Every byte in that range must be initialized and together
    /// form a valid value of `T`, including all bit-validity and pointer
    /// provenance requirements imposed by `T`.
    ///
    /// `T: Copy` does not guarantee that an arbitrary byte sequence is a valid
    /// `T`. Primitive integer and floating-point types satisfy this
    /// representation requirement; types with restricted bit patterns,
    /// references, or pointers require additional justification from the
    /// caller.
    #[must_use]
    #[inline(always)]
    pub unsafe fn read_ne_unaligned<T: Copy>(input: &[u8], index: usize) -> T {
        debug_assert!(
            Self::range_fits(input.len(), index, mem::size_of::<T>()),
            "unchecked input range exceeds source buffer"
        );
        // SAFETY: The caller guarantees byte-level validity for this unaligned
        // load.
        unsafe {
            let src = input.as_ptr().add(index).cast::<T>();
            core::ptr::read_unaligned(src)
        }
    }

    /// Writes one value to an unchecked unaligned byte slice offset.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Copyable value type whose object representation is written.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte buffer.
    /// - `index`: Byte offset in `output`.
    /// - `value`: Value to write.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested byte range does not fit.
    /// Callers do not need to repeat this range assertion.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + size_of::<T>()` is a
    /// valid writable range inside `output` and that the addition does not
    /// overflow. The complete object representation of `value`, including any
    /// padding bytes, must be initialized and valid to store in and later
    /// observe through the destination byte slice.
    ///
    /// `T: Copy` does not guarantee initialized padding or unrestricted
    /// bytewise representation. Types containing padding, references, or
    /// pointers require additional justification from the caller.
    #[inline(always)]
    pub unsafe fn write_ne_unaligned<T: Copy>(
        output: &mut [u8],
        index: usize,
        value: T,
    ) {
        debug_assert!(
            Self::range_fits(output.len(), index, mem::size_of::<T>()),
            "unchecked output range exceeds destination buffer"
        );
        // SAFETY: The caller guarantees byte-level validity for this unaligned
        // store.
        unsafe {
            let dst = output.as_mut_ptr().add(index).cast::<T>();
            core::ptr::write_unaligned(dst, value);
        }
    }
}
