// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::ptr;

/// Low-level contiguous storage with a readable window and spare tail capacity.
///
/// `Buffer` stores initialized values and tracks a readable window as
/// `data[position..limit]`. Values before `position` are considered consumed,
/// and values after `limit` are spare capacity that callers may fill before
/// advancing the limit.
///
/// This type is intentionally a low-level, hot-path API. It exposes the full
/// backing storage through [`Self::data`] and [`Self::data_mut`] so
/// higher-level buffering code can avoid repeated slicing and bounds checks.
/// Callers that mutate the backing storage directly must preserve the `position
/// <= limit <= capacity` invariant and must only make initialized spare
/// elements readable by calling [`Self::advance`].
///
/// The unsafe methods are for code that has already validated ranges at a
/// higher level. They keep debug assertions for development builds, but those
/// assertions are not a substitute for the documented safety preconditions.
///
/// # Window model
///
/// - `data[..position]` contains consumed elements.
/// - `data[position..limit]` contains readable elements.
/// - `data[limit..capacity]` is spare initialized storage.
///
/// # Examples
///
/// ```
/// use qubit_io::Buffer;
///
/// let mut buffer = Buffer::<u8>::with_capacity(4);
/// buffer.data_mut()[0..2].copy_from_slice(b"ab");
/// // SAFETY: Two initialized spare elements fit in this buffer.
/// unsafe {
///     buffer.advance(2);
/// }
///
/// assert_eq!(b"ab", &buffer.data()[buffer.position()..buffer.limit()]);
/// // SAFETY: One readable element is currently available.
/// unsafe {
///     buffer.consume(1);
/// }
/// assert_eq!(b"b", &buffer.data()[buffer.position()..buffer.limit()]);
/// ```
#[derive(Clone, Debug)]
pub struct Buffer<T>
where
    T: Copy + Default,
{
    data: Vec<T>,
    position: usize,
    limit: usize,
}

impl<T> Buffer<T>
where
    T: Copy + Default,
{
    /// Creates an empty buffer with at least the requested capacity.
    ///
    /// A requested capacity of `0` is raised to `1`.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Requested element capacity.
    ///
    /// # Returns
    ///
    /// A buffer with `position == 0` and `limit == 0`.
    #[inline(always)]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            data: vec![T::default(); capacity],
            position: 0,
            limit: 0,
        }
    }

    /// Returns the total element capacity.
    ///
    /// # Returns
    ///
    /// The length of the backing storage.
    #[inline(always)]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Returns the current readable cursor.
    ///
    /// # Returns
    ///
    /// The start index of the readable window.
    #[inline(always)]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns the current readable limit.
    ///
    /// # Returns
    ///
    /// The exclusive end index of the readable window.
    #[inline(always)]
    #[must_use]
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Returns the backing storage.
    ///
    /// # Returns
    ///
    /// The full initialized backing slice.
    #[inline(always)]
    #[must_use]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Returns the mutable backing storage.
    ///
    /// Mutating elements outside the current readable or spare operation may
    /// invalidate higher-level assumptions about buffered contents.
    ///
    /// # Returns
    ///
    /// The full initialized backing slice.
    #[inline(always)]
    #[must_use]
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Returns the number of readable elements.
    ///
    /// # Returns
    ///
    /// The length of `data[position..limit]`.
    #[inline(always)]
    #[must_use]
    pub const fn available(&self) -> usize {
        self.limit - self.position
    }

    /// Returns the number of spare elements after the limit.
    ///
    /// # Returns
    ///
    /// The length of `data[limit..]`.
    #[inline(always)]
    #[must_use]
    pub fn spare_capacity(&self) -> usize {
        self.data.len() - self.limit
    }

    /// Returns whether the readable window is empty.
    ///
    /// # Returns
    ///
    /// `true` when no elements are available for consumption.
    #[inline(always)]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.position == self.limit
    }

    /// Returns whether the spare tail is empty.
    ///
    /// # Returns
    ///
    /// `true` when `limit == capacity`.
    #[inline(always)]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.limit == self.data.len()
    }

    /// Returns raw spare-tail parts for hot-path callers.
    ///
    /// The returned slice is the full backing storage. `index` is the start of
    /// the spare window, and `count` is the number of spare elements. Callers
    /// that need a slice can use `&mut buffer[index..index + count]`; callers
    /// that already validated bounds can pass `buffer` and `index` directly to
    /// indexed unchecked operations that write from `index`.
    ///
    /// # Returns
    ///
    /// The backing storage, the spare start index, and the spare element count.
    #[inline(always)]
    #[must_use]
    pub fn spare_raw_parts_mut(&mut self) -> (&mut [T], usize, usize) {
        let index = self.limit;
        let count = self.spare_capacity();
        (self.data_mut(), index, count)
    }

    /// Clears all buffered contents.
    ///
    /// This resets both cursors to zero without modifying stored values.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.position = 0;
        self.limit = 0;
    }

    /// Advances the readable cursor without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of readable elements to consume.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.available()`.
    #[inline(always)]
    pub unsafe fn consume(&mut self, count: usize) {
        debug_assert!(
            count <= self.available(),
            "unchecked consume exceeds available buffer"
        );
        self.position += count;
    }

    /// Moves the readable cursor backward without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of already-consumed elements to make readable again.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.position()`.
    #[inline(always)]
    pub(crate) unsafe fn rewind(&mut self, count: usize) {
        debug_assert!(
            count <= self.position,
            "unchecked rewind exceeds consumed buffer prefix"
        );
        self.position -= count;
    }

    /// Advances the readable limit without checking bounds.
    ///
    /// # Parameters
    ///
    /// * `count` - Number of initialized spare elements to make readable.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `count <= self.spare_capacity()`.
    #[inline(always)]
    pub unsafe fn advance(&mut self, count: usize) {
        debug_assert!(
            count <= self.spare_capacity(),
            "unchecked advance exceeds spare buffer capacity"
        );
        self.limit += count;
    }

    /// Moves unread elements to the front of the backing storage.
    ///
    /// Consumed elements are discarded. The unread element count is preserved,
    /// and the readable window starts at zero after compaction.
    #[inline]
    pub fn compact(&mut self) {
        let available = self.available();
        if available == 0 {
            self.clear();
            return;
        }
        if self.position != 0 {
            // SAFETY: Source and destination ranges are inside the same
            // allocation and may overlap, so `copy` is used instead of
            // `copy_nonoverlapping`.
            unsafe {
                let source = self.data.as_ptr().add(self.position);
                let destination = self.data.as_mut_ptr();
                ptr::copy(source, destination, available);
            }
        }
        self.position = 0;
        self.limit = available;
    }

    /// Copies values from an external slice into the spare tail.
    ///
    /// The copied values are made readable by advancing the limit by `count`.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `input_index` - Start index inside `input`.
    /// * `count` - Number of values to copy.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input_index..input_index + count` is a
    /// valid range inside `input`, that the addition does not overflow, that
    /// `count <= self.spare_capacity()`, and that the source range does not
    /// overlap with this buffer's destination range.
    #[inline(always)]
    pub unsafe fn copy_from(&mut self, input: &[T], input_index: usize, count: usize) {
        debug_assert!(
            input_index
                .checked_add(count)
                .is_some_and(|end| end <= input.len()),
            "unchecked input range exceeds source buffer"
        );
        debug_assert!(
            count <= self.spare_capacity(),
            "unchecked input copy exceeds spare buffer capacity"
        );
        // SAFETY: The caller guarantees the source range and spare destination
        // range are valid and non-overlapping.
        unsafe {
            let source = input.as_ptr().add(input_index);
            let destination = self.data.as_mut_ptr().add(self.limit);
            ptr::copy_nonoverlapping(source, destination, count);
            self.advance(count);
        }
    }

    /// Copies readable values into an external slice.
    ///
    /// The copied values are consumed by advancing the position by `count`.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `output_index` - Start index inside `output`.
    /// * `count` - Number of values to copy.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output_index..output_index + count` is
    /// a valid range inside `output`, that the addition does not overflow, that
    /// `count <= self.available()`, and that the source range does not overlap
    /// with the destination range.
    #[inline(always)]
    pub unsafe fn copy_to(&mut self, output: &mut [T], output_index: usize, count: usize) {
        debug_assert!(
            output_index
                .checked_add(count)
                .is_some_and(|end| end <= output.len()),
            "unchecked output range exceeds destination buffer"
        );
        debug_assert!(
            count <= self.available(),
            "unchecked output copy exceeds readable buffer"
        );
        // SAFETY: The caller guarantees the readable source range and
        // destination range are valid and non-overlapping.
        unsafe {
            let source = self.data.as_ptr().add(self.position);
            let destination = output.as_mut_ptr().add(output_index);
            ptr::copy_nonoverlapping(source, destination, count);
            self.consume(count);
        }
    }
}
