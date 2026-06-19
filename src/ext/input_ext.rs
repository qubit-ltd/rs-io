// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use std::cell::Cell;
use std::io::{Error, ErrorKind, Result};

use crate::capacity_const::DEFAULT_BUFFER_CAPACITY;
use crate::ext::output_ext::OutputExt;
use crate::traits::validate_read_count;
use crate::util::{UncheckedSlice, create_vec, try_reserve_vec};
use crate::{Input, Output};

/// Extension methods for [`Input`] values.
///
/// `InputExt` keeps convenience and complete-read helpers outside the minimal
/// [`Input`] trait. The methods are implemented for every item-oriented input,
/// including `dyn Input` trait objects.
pub trait InputExt: Input {
    /// Reads exactly enough items to fill `output`.
    ///
    /// # Parameters
    /// - `output`: Destination storage to fill.
    ///
    /// # Errors
    /// Returns [`ErrorKind::UnexpectedEof`] when EOF is reached before the
    /// output slice is full. Returns the first non-[`ErrorKind::Interrupted`]
    /// error reported by the input. Interrupted reads are retried. Returns
    /// [`ErrorKind::InvalidData`] if the input reports more items than
    /// requested.
    #[inline]
    fn read_exact(&mut self, output: &mut [Self::Item]) -> Result<()> {
        let read = self.read_exact_or_eof(output)?;
        if read == output.len() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        }
    }

    /// Reads exactly `count` items into an indexed output range without
    /// checking the range bounds in release builds.
    ///
    /// This method has the same blocking and error behavior as
    /// [`InputExt::read_exact`], but writes into
    /// `output[index..index + count]` using indexed unchecked reads.
    ///
    /// # Parameters
    /// - `output`: Destination storage.
    /// - `index`: Start index inside `output`.
    /// - `count`: Number of items to read.
    ///
    /// # Errors
    /// Returns [`ErrorKind::UnexpectedEof`] when EOF is reached before the
    /// range is full. Returns the first non-[`ErrorKind::Interrupted`] error
    /// reported by the input. Interrupted reads are retried. Returns
    /// [`ErrorKind::InvalidData`] if the input reports more items than
    /// requested.
    ///
    /// # Safety
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    #[inline]
    unsafe fn read_exact_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<()> {
        let read = unsafe { self.read_exact_or_eof_unchecked(output, index, count)? };
        if read == count {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        }
    }

    /// Reads items until `output` is full or EOF is reached.
    ///
    /// This method treats EOF as a successful partial result. It keeps retrying
    /// short reads until the output slice is full, EOF is reached, or a
    /// non-interrupted error occurs.
    ///
    /// # Parameters
    /// - `output`: Destination storage to fill.
    ///
    /// # Returns
    /// The number of items written to `output`.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] error reported by the
    /// input. Interrupted reads are retried. Returns [`ErrorKind::InvalidData`]
    /// if the input reports more items than requested.
    fn read_exact_or_eof(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        let mut total = 0;
        while total < output.len() {
            let remaining = output.len() - total;
            // SAFETY: `total..output.len()` is a valid suffix of `output`.
            match unsafe { self.read_unchecked(output, total, remaining) } {
                Ok(0) => break,
                Ok(read) => {
                    validate_read_count(read, remaining)?;
                    total += read;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(total)
    }

    /// Reads items into an indexed output range until that range is full or
    /// EOF is reached, without checking the range bounds in release builds.
    ///
    /// This method has the same EOF and retry behavior as
    /// [`InputExt::read_exact_or_eof`], but writes into
    /// `output[index..index + count]` using indexed unchecked reads.
    ///
    /// # Parameters
    /// - `output`: Destination storage.
    /// - `index`: Start index inside `output`.
    /// - `count`: Number of items to try to read.
    ///
    /// # Returns
    /// The number of items written into `output[index..index + count]`. The
    /// value is in `0..=count`.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] error reported by the
    /// input. Interrupted reads are retried. Returns [`ErrorKind::InvalidData`]
    /// if the input reports more items than requested.
    ///
    /// # Safety
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_exact_or_eof_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked read range exceeds output buffer"
        );
        let mut total = 0;
        while total < count {
            let remaining = count - total;
            // SAFETY: The caller guarantees the original destination range is
            // valid; `total < count`, so this suffix remains inside it.
            match unsafe { self.read_unchecked(output, index + total, remaining) } {
                Ok(0) => break,
                Ok(read) => {
                    validate_read_count(read, remaining)?;
                    total += read;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(total)
    }

    /// Copies all remaining items from this input into `output`.
    ///
    /// The method allocates a reusable heap buffer and copies until EOF. It
    /// does not close or flush the output.
    ///
    /// # Parameters
    /// - `output`: Destination output.
    ///
    /// # Returns
    /// The number of items copied.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] read error or output
    /// error reported by the underlying streams. Interrupted reads are retried.
    /// Returns [`ErrorKind::InvalidData`] if the input reports more items than
    /// requested.
    fn copy_to<O>(&mut self, output: &mut O) -> Result<u64>
    where
        O: Output<Item = Self::Item> + ?Sized,
        Self::Item: Copy + Default,
    {
        let mut buffer = create_vec(DEFAULT_BUFFER_CAPACITY, Self::Item::default())?;
        let mut copied = 0_u64;
        loop {
            let requested = buffer.len();
            let read = read_retrying_interrupted_limited(self, &mut buffer, requested)?;
            if read == 0 {
                return Ok(copied);
            }
            // SAFETY: `read` has been validated against `buffer.len()`.
            unsafe {
                output.write_all_unchecked(&buffer, 0, read)?;
            }
            copied = add_copied(copied, read)?;
        }
    }

    /// Copies at most `max_units` items from this input into `output`.
    ///
    /// This method stops successfully when either EOF is reached or
    /// `max_units` items have been copied. It does not close or flush the
    /// output.
    ///
    /// # Parameters
    /// - `output`: Destination output.
    /// - `max_units`: Maximum number of items to copy.
    ///
    /// # Returns
    /// The number of items copied.
    ///
    /// # Errors
    /// Returns the first non-[`ErrorKind::Interrupted`] read error or output
    /// error reported by the underlying streams. Interrupted reads are retried.
    /// Returns [`ErrorKind::InvalidData`] if the input reports more items than
    /// requested.
    fn copy_to_at_most<O>(&mut self, output: &mut O, max_units: u64) -> Result<u64>
    where
        O: Output<Item = Self::Item> + ?Sized,
        Self::Item: Copy + Default,
    {
        if max_units == 0 {
            return Ok(0);
        }
        let mut buffer = create_vec(DEFAULT_BUFFER_CAPACITY, Self::Item::default())?;
        let mut remaining = max_units;
        let mut copied = 0_u64;
        while remaining > 0 {
            let requested = remaining.min(buffer.len() as u64) as usize;
            let read = read_retrying_interrupted_limited(self, &mut buffer, requested)?;
            if read == 0 {
                break;
            }
            // SAFETY: `read` has been validated against the requested prefix.
            unsafe {
                output.write_all_unchecked(&buffer, 0, read)?;
            }
            let read = read as u64;
            remaining -= read;
            copied += read;
        }
        Ok(copied)
    }

    /// Copies the remaining input if its total length is at most `max_units`.
    ///
    /// This method copies from the current input position until EOF. If EOF is
    /// not reached within `max_units` items, it returns
    /// [`ErrorKind::InvalidData`]. Detecting oversized input consumes one
    /// excess item from this input; that excess item is not written to
    /// `output`. On failure, `output` is left unchanged.
    ///
    /// # Parameters
    /// - `output`: Destination output.
    /// - `max_units`: Maximum accepted number of remaining input items.
    ///
    /// # Returns
    /// The number of items copied when EOF is reached within the limit.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidData`] when the remaining input is longer
    /// than `max_units`. Returns the first non-[`ErrorKind::Interrupted`] read
    /// error or output error reported by the underlying streams. Interrupted
    /// reads are retried. Returns [`ErrorKind::InvalidData`] if the input
    /// reports more items than requested.
    fn copy_to_end_limited<O>(&mut self, output: &mut O, max_units: u64) -> Result<u64>
    where
        O: Output<Item = Self::Item> + ?Sized,
        Self::Item: Copy + Default,
    {
        let mut buffer = create_vec(DEFAULT_BUFFER_CAPACITY, Self::Item::default())?;
        let mut collected = Vec::new();
        let mut remaining = max_units;
        let mut copied = 0_u64;
        loop {
            let requested = remaining.saturating_add(1).min(buffer.len() as u64) as usize;
            let read = read_retrying_interrupted_limited(self, &mut buffer, requested)?;
            if read == 0 {
                if !collected.is_empty() {
                    // SAFETY: `collected` contains exactly the items validated
                    // below.
                    unsafe {
                        output.write_all_unchecked(&collected, 0, collected.len())?;
                    }
                }
                return Ok(copied);
            }
            if (read as u64) > remaining {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("input exceeds maximum length of {max_units} items"),
                ));
            }
            try_reserve_vec(&mut collected, read)?;
            collected.extend_from_slice(&buffer[..read]);
            let read = read as u64;
            remaining -= read;
            copied = add_copied(copied, read as usize)?;
        }
    }
}

impl<T> InputExt for T where T: Input + ?Sized {}

/// Reads into a buffer prefix while retrying interrupted reads.
///
/// # Parameters
/// - `input`: Source input.
/// - `buffer`: Destination buffer.
/// - `requested`: Number of items to request.
///
/// # Returns
/// The number of items read.
///
/// # Errors
/// Returns the first non-interrupted input error. Returns
/// [`ErrorKind::InvalidData`] if the input reports more items than requested.
fn read_retrying_interrupted_limited<I>(
    input: &mut I,
    buffer: &mut [I::Item],
    requested: usize,
) -> Result<usize>
where
    I: Input + ?Sized,
{
    loop {
        // SAFETY: Callers pass a valid prefix length within `buffer`.
        match unsafe { input.read_unchecked(buffer, 0, requested) } {
            Ok(read) => {
                validate_read_count(read, requested)?;
                return Ok(read);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] if the count overflows `u64`.
#[cfg(coverage)]
thread_local! {
    static COVERAGE_FAIL_NEXT_ADD_COPIED: Cell<bool> = const { Cell::new(false) };
}

/// Makes the next [`add_copied`] call fail.
///
/// Coverage-only helper for exercising overflow propagation inside copy loops.
#[cfg(coverage)]
pub fn coverage_fail_next_add_copied() {
    COVERAGE_FAIL_NEXT_ADD_COPIED.with(|state| state.set(true));
}

/// Clears coverage-only add_copied hooks between tests.
#[cfg(coverage)]
pub fn coverage_reset_add_copied_hooks() {
    COVERAGE_FAIL_NEXT_ADD_COPIED.with(|state| state.set(false));
}

/// Adds a copied item count to an accumulated total.
///
/// # Parameters
/// - `copied`: Existing copied item count.
/// - `read`: Newly copied item count.
///
/// # Returns
/// The updated copied item count.
///
/// # Errors
/// Returns [`ErrorKind::InvalidData`] if the count overflows `u64`.
#[inline(always)]
fn add_copied(copied: u64, read: usize) -> Result<u64> {
    #[cfg(coverage)]
    if COVERAGE_FAIL_NEXT_ADD_COPIED.with(|state| {
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
    copied
        .checked_add(read as u64)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "copied item count overflows u64"))
}
