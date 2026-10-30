// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal read and refill stages for [`super::BufferedInput`].

use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;

use crate::Buffer;
use crate::Input;
use crate::traits::validate_read_count;
use crate::util::SliceRange;

/// Appends one chunk from a type-erased input to a buffer.
///
/// Interrupted reads are retried until another outcome is observed.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies the next chunk.
/// - `buffer`: Buffer whose spare tail receives the chunk.
///
/// # Returns
///
/// Returns `Ok(true)` when at least one item is appended, or `Ok(false)` at
/// EOF.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if the input reports more items than the
/// spare tail can hold. Other non-interrupted errors are propagated from the
/// input.
///
/// # Panics
///
/// Panics in debug builds if `buffer` has no spare capacity.
pub(super) fn read_more_impl<T>(inner: &mut dyn Input<Item = T>, buffer: &mut Buffer<T>) -> Result<bool>
where
    T: Clone + Default,
{
    let count = buffer.spare_capacity();
    debug_assert!(count > 0, "buffer has no tail capacity");
    loop {
        let limit = buffer.limit();
        // SAFETY: `limit` is within `buffer`, and `count` is the remaining
        // capacity from `limit` to the end of `buffer`.
        match unsafe { inner.read_unchecked(buffer.data_mut(), limit, count) } {
            Ok(0) => return Ok(false),
            Ok(read) => {
                validate_read_count(read, count)?;
                // SAFETY: The validated count fits the spare buffer range.
                unsafe {
                    buffer.advance(read);
                }
                return Ok(true);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Preserves unread items and appends one chunk from a type-erased input.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies the next chunk.
/// - `buffer`: Buffer whose unread items must be preserved.
///
/// # Returns
///
/// Returns `Ok(true)` when at least one item is appended, or `Ok(false)` at
/// EOF.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when the buffer is full and has no
/// consumed prefix to reclaim. Returns [`ErrorKind::InvalidData`] for an
/// invalid count reported by `inner`; other non-interrupted errors are
/// propagated.
pub(in crate::buffered) fn fill_more_impl<T>(inner: &mut dyn Input<Item = T>, buffer: &mut Buffer<T>) -> Result<bool>
where
    T: Clone + Default,
{
    if buffer.available() == 0 {
        buffer.clear();
    } else if buffer.spare_capacity() == 0 {
        buffer.compact();
        if buffer.spare_capacity() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "buffered input is full; consume buffered items before refilling",
            ));
        }
    }
    read_more_impl(inner, buffer)
}

/// Refills a buffer to a requested unread item count.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies additional items.
/// - `buffer`: Buffer retaining unread items.
/// - `count`: Required unread item count.
///
/// # Returns
///
/// Returns `Ok(true)` when `count` items are available, or `Ok(false)` when EOF
/// is reached first.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when `count` exceeds the buffer
/// capacity. Returns [`ErrorKind::InvalidData`] for an invalid count reported
/// by `inner`; other non-interrupted input errors are propagated.
pub(in crate::buffered) fn fill_until_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    count: usize,
) -> Result<bool>
where
    T: Clone + Default,
{
    if count > buffer.capacity() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "requested available items exceed buffered input capacity",
        ));
    }
    while buffer.available() < count {
        let available = buffer.available();
        if available == 0 {
            buffer.clear();
        } else {
            let missing = count - available;
            if buffer.spare_capacity() < missing {
                buffer.compact();
            }
        }
        if !read_more_impl(inner, buffer)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Ensures a requested unread item count through a type-erased input.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies additional items.
/// - `buffer`: Buffer retaining unread items.
/// - `count`: Required unread item count.
///
/// # Returns
///
/// Returns `Ok(())` when `count` unread items are available.
///
/// # Errors
///
/// Returns [`ErrorKind::UnexpectedEof`] when EOF is reached first,
/// [`ErrorKind::InvalidInput`] when `count` exceeds the buffer capacity, or
/// [`ErrorKind::InvalidData`] for an invalid count reported by `inner`. Other
/// non-interrupted input errors are propagated.
pub(in crate::buffered) fn ensure_available_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    count: usize,
) -> Result<()>
where
    T: Clone + Default,
{
    if fill_until_impl(inner, buffer, count)? {
        return Ok(());
    }
    let available = buffer.available();
    // SAFETY: `available` is the current readable item count.
    unsafe {
        buffer.consume(available);
    }
    Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
}

/// Reads an indexed range through a type-erased input and retained buffer.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies items when the buffer is empty.
/// - `buffer`: Buffer retaining unread items.
/// - `output`: Destination item slice.
/// - `output_index`: Starting destination index.
/// - `count`: Maximum number of items to read.
///
/// # Returns
///
/// Returns the number of items copied or read.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if `inner` reports more than `count`
/// items. Other errors are propagated from `inner`.
///
/// # Panics
///
/// Panics in debug builds if the requested output range does not fit.
///
/// # Safety
///
/// The caller must guarantee that `output_index..output_index + count` is a
/// valid range inside `output` and that the addition does not overflow.
pub(in crate::buffered) unsafe fn read_unchecked_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize>
where
    T: Clone + Default,
{
    debug_assert!(
        SliceRange::range_fits(output.len(), output_index, count),
        "unchecked read output range exceeds destination buffer"
    );
    if count == 0 {
        return Ok(0);
    }
    if buffer.available() == 0 {
        buffer.clear();
        if count >= buffer.capacity() {
            // SAFETY: Forwarded from the caller.
            let read = unsafe { inner.read_unchecked(output, output_index, count) }?;
            validate_read_count(read, count)?;
            return Ok(read);
        }
        if !read_more_impl(inner, buffer)? {
            return Ok(0);
        }
    }
    let read_count = count.min(buffer.available());
    // SAFETY: The count fits the output range and readable buffer window.
    unsafe {
        buffer.copy_to(output, output_index, read_count);
    }
    Ok(read_count)
}

/// Clones the available buffered prefix into an output range.
///
/// Returns the number of cloned items. The caller must guarantee that the
/// output range contains at least `count` items after `output_index`.
///
/// # Safety
///
/// `output_index..output_index + count` must be a valid range in `output`,
/// and the addition must not overflow.
unsafe fn copy_available_to_output<T>(
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> usize
where
    T: Clone + Default,
{
    let copied = count.min(buffer.available());
    if copied != 0 {
        // SAFETY: `copied` fits both the readable window and caller output.
        unsafe {
            buffer.copy_to(output, output_index, copied);
        }
    }
    copied
}

/// Reads a requested suffix directly from the wrapped input, retrying on
/// interruptions.
///
/// Returns the validated item count reported by the wrapped input. The caller
/// must guarantee that the indexed output range is valid.
///
/// # Safety
///
/// `output_index..output_index + count` must be a valid range in `output`,
/// and the addition must not overflow.
unsafe fn read_direct_fully<T>(
    inner: &mut dyn Input<Item = T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize> {
    loop {
        // SAFETY: Forwarded from the caller.
        match unsafe { inner.read_fully_unchecked(output, output_index, count) } {
            Ok(read) => {
                validate_read_count(read, count)?;
                return Ok(read);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Reads a requested suffix through the internal buffer until EOF or full.
///
/// Returns the accumulated number of items read. The caller must guarantee
/// that the indexed output range is valid.
///
/// # Safety
///
/// `output_index..output_index + count` must be a valid range in `output`,
/// and each offset added to `output_index` must remain in that range.
unsafe fn read_buffered_remainder<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize>
where
    T: Clone + Default,
{
    let mut total = 0;
    while total < count {
        let remaining = count - total;
        // SAFETY: `total` stays below `count`, so the suffix fits caller
        // output.
        match unsafe { read_unchecked_impl(inner, buffer, output, output_index + total, remaining) } {
            Ok(0) => break,
            Ok(read) => total += read,
            Err(error) => return Err(error),
        }
    }
    Ok(total)
}

/// Fills an indexed range through a type-erased input and retained buffer.
///
/// # Type Parameters
///
/// - `T`: Cloneable input item type.
///
/// # Parameters
///
/// - `inner`: Input that supplies items after buffered items are consumed.
/// - `buffer`: Buffer retaining unread items.
/// - `output`: Destination item slice.
/// - `output_index`: Starting destination index.
/// - `count`: Number of items requested.
///
/// # Returns
///
/// Returns the number of items read before the range filled or EOF was
/// reached.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if `inner` reports more items than
/// requested. Other non-interrupted errors are propagated from `inner`.
///
/// # Panics
///
/// Panics in debug builds if the requested output range does not fit.
///
/// # Safety
///
/// The caller must guarantee that `output_index..output_index + count` is a
/// valid range inside `output` and that the addition does not overflow.
pub(in crate::buffered) unsafe fn read_fully_unchecked_impl<T>(
    inner: &mut dyn Input<Item = T>,
    buffer: &mut Buffer<T>,
    output: &mut [T],
    output_index: usize,
    count: usize,
) -> Result<usize>
where
    T: Clone + Default,
{
    debug_assert!(
        SliceRange::range_fits(output.len(), output_index, count),
        "unchecked read-fully output range exceeds destination buffer"
    );
    if count == 0 {
        return Ok(0);
    }

    // SAFETY: The caller guarantees the complete output range is valid.
    let total = unsafe { copy_available_to_output(buffer, output, output_index, count) };
    if total == count {
        return Ok(total);
    }

    let remaining = count - total;
    if remaining >= buffer.capacity() {
        buffer.clear();
        // SAFETY: The remaining suffix is inside the caller's output range.
        let read = unsafe { read_direct_fully(inner, output, output_index + total, remaining) }?;
        return Ok(total + read);
    }

    // SAFETY: The remaining suffix is inside the caller's output range.
    let read = unsafe { read_buffered_remainder(inner, buffer, output, output_index + total, remaining) }?;
    Ok(total + read)
}
