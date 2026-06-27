// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
};

use crate::util::UncheckedSlice;

/// Minimal indexed input interface over items.
///
/// `Input` is intentionally smaller and lower-level than [`Read`]. It only
/// states that an implementor can read up to `count` items into
/// `output[index..index + count]`. The caller owns range validation so hot
/// paths can avoid repeated slicing and bounds checks.
///
/// # Coherence note
///
/// Every [`Read`] value automatically implements `Input<Item = u8>` through the
/// blanket impl below. Because [`Input::Item`] is an associated type rather than
/// a trait parameter, a concrete type that implements [`Read`] cannot also
/// provide any other direct `Input` implementation for the same type, including
/// one with a different item type.
///
/// Use a wrapper/newtype when a type needs item-oriented input semantics that
/// differ from its byte-oriented [`Read`] implementation.
pub trait Input {
    /// The item type read from this input.
    type Item;

    /// Returns whether this input already buffers items internally.
    ///
    /// # Returns
    ///
    /// `true` when callers should avoid wrapping this input in another generic
    /// item buffer automatically.
    #[inline(always)]
    #[must_use]
    fn is_buffered(&self) -> bool {
        false
    }

    /// Reads items into an indexed output range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// The number of items written into `output[index..index + count]`. The
    /// value must be in `0..=count`.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Reads items into the full output slice.
    ///
    /// # Parameters
    /// - `output`: Destination storage.
    ///
    /// # Returns
    /// The number of items read into `output`.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation. Returns
    /// [`ErrorKind::InvalidData`] if the implementation reports reading more
    /// items than requested.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        // SAFETY: The caller ensured the destination slice is valid.
        let read = unsafe { self.read_unchecked(output, 0, output.len()) }?;
        validate_read_count(read, output.len())?;
        Ok(read)
    }

    /// Reads items into an indexed output range until it is full or EOF is
    /// reached.
    ///
    /// This method retries interrupted reads and treats EOF as a successful
    /// partial result.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// The number of items written into `output[index..index + count]`.
    ///
    /// # Errors
    ///
    /// Returns the first non-[`ErrorKind::Interrupted`] input error. Returns
    /// [`ErrorKind::InvalidData`] if the implementation reports more items than
    /// requested.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_fully_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked read-fully range exceeds output buffer"
        );
        let mut total = 0;
        while total < count {
            let remaining = count - total;
            // SAFETY: The caller guarantees the original destination range is
            // valid; `total < count`, so this suffix remains inside it.
            match unsafe {
                self.read_unchecked(output, index + total, remaining)
            } {
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

    /// Reads items into the full output slice until it is full or EOF is
    /// reached.
    ///
    /// # Parameters
    /// - `output`: Destination storage to fill as far as possible.
    ///
    /// # Returns
    /// The number of items written into `output`.
    ///
    /// # Errors
    /// Returns the first non-interrupted input error, or
    /// [`ErrorKind::InvalidData`] if the implementation reports an impossible
    /// item count.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        // SAFETY: The full output slice is a valid destination range.
        unsafe { self.read_fully_unchecked(output, 0, output.len()) }
    }
}

impl<R> Input for R
where
    R: Read + ?Sized,
{
    type Item = u8;

    /// Reads bytes from a standard [`Read`] value into an indexed range.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(output.len(), index, count),
            "unchecked read range exceeds output buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `output`.
        let target =
            unsafe { UncheckedSlice::subslice_mut(output, index, count) };
        Read::read(self, target)
    }

    /// Reads items into the full output slice.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        Read::read(self, output)
    }
}

/// Validates an item count returned by an input.
///
/// # Parameters
///
/// * `read` - Item count reported by the input.
/// * `requested` - Maximum item count requested from the input.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidData`] when the input reports more items than
/// the destination range could hold.
#[inline(always)]
pub fn validate_read_count(read: usize, requested: usize) -> Result<()> {
    if read > requested {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "reader reported {read} items for a {requested}-item buffer"
            ),
        ));
    }
    Ok(())
}
