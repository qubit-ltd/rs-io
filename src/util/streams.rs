// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Copy and comparison operations for byte and item streams.

// qubit-style: allow coverage-cfg
#[cfg(coverage)]
use std::cell::Cell;
use std::cmp::Ordering;
use std::convert::Infallible;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};

use crate::capacity_const::{
    DEFAULT_BUFFER_CAPACITY,
    DEFAULT_COMPARE_BUFFER_SIZE,
    DEFAULT_COPY_BUFFER_SIZE,
};
use crate::std_io::ext::ReadExt;
use crate::traits::validate_read_count;
use crate::util::{
    allocation_error,
    create_vec,
    try_reserve_vec,
};
use crate::{
    Input,
    Output,
};

/// Stream utility namespace.
///
/// This type is an uninstantiable namespace for operations involving one or
/// more [`Read`] or [`Write`] values. The methods do not close or flush the
/// supplied streams unless the underlying standard-library operation documents
/// otherwise.
///
/// # Examples
/// ```
/// use qubit_io::Streams;
/// use std::io::Cursor;
///
/// let mut input = Cursor::new(b"abcdef".to_vec());
/// let mut output = Vec::new();
///
/// let copied = Streams::copy_at_most(&mut input, &mut output, 4)?;
///
/// assert_eq!(4, copied);
/// assert_eq!(b"abcd", output.as_slice());
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct Streams {
    /// Prevents construction of this namespace type.
    _private: Infallible,
}

impl Streams {
    /// Copies all remaining bytes from `reader` to `writer`.
    ///
    /// This is a namespace-style wrapper around [`std::io::copy`]. It preserves
    /// the standard-library behavior, including platform-specific optimized
    /// copy paths when available.
    ///
    /// # Type Parameters
    /// - `R`: Source reader type.
    /// - `W`: Destination writer type.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns the first read or write error reported by the underlying
    /// streams, using the same error behavior as [`std::io::copy`].
    #[inline(always)]
    pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        std::io::copy(reader, writer)
    }

    /// Copies at most `max_bytes` bytes from `reader` to `writer`.
    ///
    /// This method stops successfully when either EOF is reached or
    /// `max_bytes` bytes have been copied. It does not close or flush either
    /// stream.
    ///
    /// # Type Parameters
    /// - `R`: Source reader type.
    /// - `W`: Destination writer type.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum number of bytes to copy.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns [`ErrorKind::OutOfMemory`] if the temporary copy buffer cannot
    /// be allocated. Returns the first non-interrupted read error or write
    /// error reported by the underlying streams. Interrupted reads are retried.
    #[inline(always)]
    pub fn copy_at_most<R, W>(
        reader: &mut R,
        writer: &mut W,
        max_bytes: u64,
    ) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_at_most_impl(
            &mut reader,
            &mut writer,
            max_bytes,
            DEFAULT_COPY_BUFFER_SIZE,
        )
    }

    /// Copies at most `max_bytes` bytes from `reader` to `writer` using a
    /// caller-selected maximum heap-buffer size.
    ///
    /// This method has the same copy semantics as [`Self::copy_at_most`], but
    /// allocates up to `buffer_size` bytes, capped by `max_bytes`. Use it to
    /// control temporary heap usage and read granularity.
    ///
    /// # Type Parameters
    /// - `R`: Source reader type.
    /// - `W`: Destination writer type.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum number of bytes to copy.
    /// - `buffer_size`: Maximum number of bytes in the copy buffer.
    ///
    /// # Returns
    /// The number of bytes copied.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
    /// [`ErrorKind::OutOfMemory`] error if the copy buffer cannot be allocated.
    /// Returns the first non-interrupted read error or write error reported by
    /// the underlying streams. Interrupted reads are retried.
    #[inline(always)]
    pub fn copy_at_most_with_buffer_size<R, W>(
        reader: &mut R,
        writer: &mut W,
        max_bytes: u64,
        buffer_size: usize,
    ) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_at_most_impl(&mut reader, &mut writer, max_bytes, buffer_size)
    }

    /// Copies the remaining input if its total length is at most `max_bytes`.
    ///
    /// This method copies from the current reader position until EOF. If EOF is
    /// not reached within `max_bytes` bytes, it returns
    /// [`std::io::ErrorKind::InvalidData`]. Detecting oversized input consumes
    /// one excess byte from `reader`; that excess byte is not written to
    /// `writer`.
    ///
    /// Unlike bounded reads into in-memory collections, this method cannot roll
    /// back bytes already accepted by `writer` when the limit is exceeded
    /// because [`Write`] does not provide truncation. On
    /// [`std::io::ErrorKind::InvalidData`], up to `max_bytes` bytes may remain
    /// in `writer`.
    ///
    /// # Type Parameters
    /// - `R`: Source reader type.
    /// - `W`: Destination writer type.
    ///
    /// # Parameters
    /// - `reader`: Source reader.
    /// - `writer`: Destination writer.
    /// - `max_bytes`: Maximum accepted number of bytes in the remaining input.
    ///
    /// # Returns
    /// The number of bytes copied when EOF is reached within the limit.
    ///
    /// # Errors
    /// Returns [`std::io::ErrorKind::InvalidData`] when the remaining input is
    /// longer than `max_bytes`. Returns [`ErrorKind::OutOfMemory`] if the
    /// temporary copy buffer cannot be allocated. Returns the first
    /// non-interrupted read error or write error reported by the underlying
    /// streams. Interrupted reads are retried.
    #[inline(always)]
    pub fn copy_to_end_limited<R, W>(
        reader: &mut R,
        writer: &mut W,
        max_bytes: u64,
    ) -> Result<u64>
    where
        R: Read + ?Sized,
        W: Write + ?Sized,
    {
        let mut reader = reader;
        let mut writer = writer;
        copy_to_end_limited_impl(&mut reader, &mut writer, max_bytes)
    }

    /// Copies all remaining items from `input` to `output`.
    ///
    /// This method allocates a reusable item buffer and copies until EOF. It
    /// does not close or flush `output`.
    ///
    /// # Type Parameters
    /// - `T`: Cloneable item type shared by the input and output.
    ///
    /// # Parameters
    /// - `input`: Source item input.
    /// - `output`: Destination item output.
    ///
    /// # Returns
    /// The number of items copied.
    ///
    /// # Errors
    /// Returns [`ErrorKind::OutOfMemory`] if the reusable item buffer cannot be
    /// allocated. Returns the first non-interrupted read error or output error
    /// reported by the underlying streams. Returns [`ErrorKind::InvalidData`]
    /// if an input or output reports an impossible item count or the
    /// accumulated copied item count overflows `u64`.
    ///
    /// # Panics
    /// Panics if `T::default()` or `T::clone()` panics.
    pub fn copy_input_to_output<T>(
        input: &mut dyn Input<Item = T>,
        output: &mut dyn Output<Item = T>,
    ) -> Result<u64>
    where
        T: Clone + Default,
    {
        let mut buffer = create_vec(DEFAULT_BUFFER_CAPACITY, T::default())?;
        let mut copied = 0_u64;
        loop {
            let read = input.read_fully(&mut buffer)?;
            validate_read_count(read, buffer.len())?;
            if read == 0 {
                return Ok(copied);
            }
            // SAFETY: `read` is bounded by `buffer.len()`.
            unsafe {
                output.write_fully_unchecked(&buffer, 0, read)?;
            }
            copied = add_item_count(copied, read)?;
        }
    }

    /// Copies at most `max_items` items from `input` to `output`.
    ///
    /// This method stops successfully when either EOF is reached or `max_items`
    /// items have been copied. It does not close or flush `output`.
    ///
    /// # Type Parameters
    /// - `T`: Cloneable item type shared by the input and output.
    ///
    /// # Parameters
    /// - `input`: Source item input.
    /// - `output`: Destination item output.
    /// - `max_items`: Maximum number of items to copy.
    ///
    /// # Returns
    /// The number of items copied.
    ///
    /// # Errors
    /// Returns [`ErrorKind::OutOfMemory`] if the reusable item buffer cannot be
    /// allocated. Returns the first non-interrupted read error or output error
    /// reported by the underlying streams. Returns [`ErrorKind::InvalidData`]
    /// if an input or output reports an impossible item count.
    ///
    /// # Panics
    /// Panics if `max_items > 0` and `T::default()` or `T::clone()` panics.
    pub fn copy_input_to_output_at_most<T>(
        input: &mut dyn Input<Item = T>,
        output: &mut dyn Output<Item = T>,
        max_items: u64,
    ) -> Result<u64>
    where
        T: Clone + Default,
    {
        if max_items == 0 {
            return Ok(0);
        }
        let buffer_size = usize::try_from(max_items)
            .unwrap_or(usize::MAX)
            .min(DEFAULT_BUFFER_CAPACITY);
        let mut buffer = create_vec(buffer_size, T::default())?;
        let mut remaining = max_items;
        let mut copied = 0_u64;
        while remaining > 0 {
            let requested = remaining.min(buffer.len() as u64) as usize;
            // SAFETY: `requested` is a valid prefix length inside `buffer`.
            let read = unsafe {
                input.read_fully_unchecked(&mut buffer, 0, requested)?
            };
            validate_read_count(read, requested)?;
            if read == 0 {
                break;
            }
            // SAFETY: `read` is bounded by the requested prefix.
            unsafe {
                output.write_fully_unchecked(&buffer, 0, read)?;
            }
            let read = read as u64;
            remaining -= read;
            copied = add_item_count(copied, read as usize)?;
        }
        Ok(copied)
    }

    /// Copies the remaining input if its total length is at most `max_items`.
    ///
    /// This method copies from the current input position until EOF. If EOF is
    /// not reached within `max_items` items, it returns
    /// [`ErrorKind::InvalidData`]. Detecting oversized input consumes one
    /// excess item from `input`; that excess item is not written to
    /// `output`.
    ///
    /// Oversized input, read errors, and allocation failures before output
    /// flushing leave `output` unchanged. Once EOF is reached and collected
    /// items are written to `output`, a write error may leave partial items
    /// accepted by `output` because [`Output`] has no rollback operation.
    /// Preserving this behavior requires retaining all accepted items until EOF
    /// is observed, so temporary memory usage can grow to `max_items` elements.
    ///
    /// # Type Parameters
    /// - `T`: Cloneable item type shared by the input and output.
    ///
    /// # Parameters
    /// - `input`: Source item input.
    /// - `output`: Destination item output.
    /// - `max_items`: Maximum accepted number of remaining input items.
    ///
    /// # Returns
    /// The number of items copied when EOF is reached within the limit.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the remaining input is longer
    /// than `max_items`, or when an input reports an impossible item count.
    /// Returns [`ErrorKind::OutOfMemory`] if the temporary read buffer or
    /// retained item collection cannot be allocated. Returns the first
    /// non-interrupted read error or output error reported by the underlying
    /// streams.
    ///
    /// # Panics
    /// Panics if `T::default()` or `T::clone()` panics.
    pub fn copy_input_to_output_end_limited<T>(
        input: &mut dyn Input<Item = T>,
        output: &mut dyn Output<Item = T>,
        max_items: u64,
    ) -> Result<u64>
    where
        T: Clone + Default,
    {
        let buffer_size = usize::try_from(max_items.saturating_add(1))
            .unwrap_or(usize::MAX)
            .min(DEFAULT_BUFFER_CAPACITY);
        let mut buffer = create_vec(buffer_size, T::default())?;
        let mut collected = Vec::new();
        let mut remaining = max_items;
        let mut copied = 0_u64;
        loop {
            let requested =
                remaining.saturating_add(1).min(buffer.len() as u64) as usize;
            // SAFETY: `requested` is a valid prefix length inside `buffer`.
            let read = unsafe {
                input.read_fully_unchecked(&mut buffer, 0, requested)?
            };
            validate_read_count(read, requested)?;
            if read == 0 {
                let count = collected.len();
                if count == 0 {
                    return Ok(copied);
                }
                // SAFETY: The full collected range is valid.
                unsafe {
                    output.write_fully_unchecked(&collected, 0, count)?;
                }
                return Ok(copied);
            }
            if (read as u64) > remaining {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "input exceeds maximum length of {max_items} items"
                    ),
                ));
            }
            try_reserve_vec(&mut collected, read).map_err(allocation_error)?;
            collected.extend_from_slice(&buffer[..read]);
            let read = read as u64;
            remaining -= read;
            copied = add_item_count(copied, read as usize)?;
        }
    }

    /// Tests whether two readable streams have equal remaining contents.
    ///
    /// The comparison starts at each reader's current position and reads both
    /// streams in fixed-size chunks. A mismatch stops comparison immediately
    /// after the differing chunks are read, so each reader may have advanced
    /// past the first differing byte within that chunk.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    ///
    /// # Returns
    /// `true` when both streams produce the same bytes until EOF.
    ///
    /// # Errors
    /// Returns [`ErrorKind::OutOfMemory`] if the comparison buffers cannot be
    /// allocated. Returns the first read error reported by either stream.
    #[inline(always)]
    pub fn content_eq(
        left: &mut dyn Read,
        right: &mut dyn Read,
    ) -> Result<bool> {
        Ok(Self::compare_content(left, right)? == Ordering::Equal)
    }

    /// Lexicographically compares the remaining contents of two readable
    /// streams.
    ///
    /// The comparison starts at each reader's current position and reads both
    /// streams in fixed-size chunks. A mismatch stops comparison immediately
    /// after the differing chunks are read, so each reader may have advanced
    /// past the first differing byte within that chunk.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    ///
    /// # Returns
    /// The lexicographic ordering of the remaining bytes.
    ///
    /// # Errors
    /// Returns [`ErrorKind::OutOfMemory`] if the comparison buffers cannot be
    /// allocated. Returns the first read error reported by either stream.
    #[inline(always)]
    pub fn compare_content(
        left: &mut dyn Read,
        right: &mut dyn Read,
    ) -> Result<Ordering> {
        Self::compare_content_with_buffer_size(
            left,
            right,
            DEFAULT_COMPARE_BUFFER_SIZE,
        )
    }

    /// Lexicographically compares the remaining contents of two readable
    /// streams using caller-selected heap buffers.
    ///
    /// This method has the same comparison and stream-advance semantics as
    /// [`Self::compare_content`], but allocates two buffers on the heap with
    /// `buffer_size` bytes each. Use it to control temporary heap usage and
    /// read granularity.
    ///
    /// # Parameters
    /// - `left`: First stream.
    /// - `right`: Second stream.
    /// - `buffer_size`: Number of bytes in each comparison buffer.
    ///
    /// # Returns
    /// The lexicographic ordering of the remaining bytes.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
    /// [`ErrorKind::OutOfMemory`] error if the comparison buffers cannot be
    /// allocated. Returns the first read error reported by either stream.
    ///
    /// # Panics
    /// Panics in debug builds if the internally allocated comparison buffers
    /// do not have the requested equal, nonzero length.
    pub fn compare_content_with_buffer_size(
        left: &mut dyn Read,
        right: &mut dyn Read,
        buffer_size: usize,
    ) -> Result<Ordering> {
        if buffer_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "compare buffer size must be greater than zero",
            ));
        }
        let mut left_buffer = create_vec(buffer_size, 0)?;
        let mut right_buffer = create_vec(buffer_size, 0)?;
        debug_assert_eq!(
            left_buffer.len(),
            right_buffer.len(),
            "compare buffers must have identical lengths",
        );
        debug_assert!(
            !left_buffer.is_empty(),
            "compare buffers must not be empty",
        );
        loop {
            let left_count = left.read_exact_or_eof(&mut left_buffer)?;
            let right_count = right.read_exact_or_eof(&mut right_buffer)?;
            let n = left_count.min(right_count);
            match left_buffer[..n].cmp(&right_buffer[..n]) {
                Ordering::Equal => {}
                ordering => return Ok(ordering),
            }
            match left_count.cmp(&right_count) {
                Ordering::Equal if left_count == 0 => {
                    return Ok(Ordering::Equal);
                }
                Ordering::Equal => {}
                ordering => return Ok(ordering),
            }
        }
    }
}

/// Copies at most `max_bytes` bytes using trait-object I/O endpoints.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `writer`: Destination writer.
/// - `max_bytes`: Maximum number of bytes to copy.
/// - `buffer_size`: Number of bytes in the copy buffer.
///
/// # Returns
/// The number of bytes copied.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `buffer_size == 0`. Returns an
/// [`ErrorKind::OutOfMemory`] error if the copy buffer cannot be allocated.
/// Returns the first non-interrupted read error or write error reported by the
/// underlying streams. Interrupted reads are retried.
fn copy_at_most_impl(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    max_bytes: u64,
    buffer_size: usize,
) -> Result<u64> {
    if buffer_size == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "copy buffer size must be greater than zero",
        ));
    }
    if max_bytes == 0 {
        return Ok(0);
    }
    let buffer_size = usize::try_from(max_bytes)
        .unwrap_or(usize::MAX)
        .min(buffer_size);
    let mut buffer = create_vec(buffer_size, 0)?;
    let mut remaining = max_bytes;
    let mut copied = 0;
    while remaining > 0 {
        let requested = remaining.min(buffer_size as u64) as usize;
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => break,
            Ok(count) => {
                writer.write_all(&buffer[..count])?;
                let count = count as u64;
                remaining -= count;
                copied += count;
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Ok(copied)
}

/// Copies an entire bounded byte stream through type-erased endpoints.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `writer`: Destination writer.
/// - `max_bytes`: Maximum accepted number of remaining bytes.
///
/// # Returns
/// The number of bytes copied when EOF is reached within the limit.
///
/// # Errors
/// Returns [`ErrorKind::OutOfMemory`] if the temporary copy buffer cannot be
/// allocated, a read or write error, or [`ErrorKind::InvalidData`] when the
/// remaining input exceeds `max_bytes`.
fn copy_to_end_limited_impl(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    max_bytes: u64,
) -> Result<u64> {
    let copied =
        copy_at_most_impl(reader, writer, max_bytes, DEFAULT_COPY_BUFFER_SIZE)?;
    if copied < max_bytes {
        return Ok(copied);
    }
    let mut byte = [0];
    loop {
        match reader.read(&mut byte) {
            Ok(0) => return Ok(copied),
            Ok(_) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "input exceeds maximum length of {max_bytes} bytes"
                    ),
                ));
            }
            Err(error) => {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
        }
    }
}

#[cfg(coverage)]
thread_local! {
    /// Whether the next copied-item count update should fail.
    static COVERAGE_FAIL_NEXT_ADD_ITEM_COUNT: Cell<bool> = const { Cell::new(false) };
}

/// Makes the next [`add_item_count`] call fail.
///
/// Coverage-only helper for exercising overflow propagation inside copy loops.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_fail_next_add_item_count() {
    COVERAGE_FAIL_NEXT_ADD_ITEM_COUNT.with(|state| state.set(true));
}

/// Clears coverage-only [`add_item_count`] hooks between tests.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_reset_add_item_count_hooks() {
    COVERAGE_FAIL_NEXT_ADD_ITEM_COUNT.with(|state| state.set(false));
}

/// Adds a copied item count to an accumulated total.
///
/// # Parameters
/// - `copied`: Existing copied item count.
/// - `count`: Newly copied item count.
///
/// # Returns
/// The updated copied item count.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] if the count overflows `u64`.
#[inline]
fn add_item_count(copied: u64, count: usize) -> Result<u64> {
    #[cfg(coverage)]
    if COVERAGE_FAIL_NEXT_ADD_ITEM_COUNT.with(|state| {
        let fail = state.get();
        if fail {
            state.set(false);
        }
        fail
    }) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "copied item count overflows u64",
        ));
    }
    copied.checked_add(count as u64).ok_or_else(|| {
        Error::new(ErrorKind::InvalidData, "copied item count overflows u64")
    })
}

/// Exercises the copied-item overflow branch in coverage builds.
///
/// # Returns
///
/// The overflow error returned by [`add_item_count`].
///
/// # Errors
///
/// Always returns [`ErrorKind::InvalidData`] for the forced overflow.
#[cfg(coverage)]
#[doc(hidden)]
#[inline(always)]
pub fn coverage_add_item_count_overflow() -> Result<u64> {
    add_item_count(u64::MAX, 1)
}
