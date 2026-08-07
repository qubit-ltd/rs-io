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
    Result,
};

use super::validate_write_count;
use crate::util::SliceRange;

/// Minimal indexed output interface over items.
///
/// `Output` is intentionally smaller and lower-level than [`std::io::Write`].
/// Its unchecked primitive writes up to `count` items from `input[index..index
/// + count]`, plus an explicit flush operation. The caller owns range
/// validation for unchecked calls so hot paths can avoid repeated slicing and
/// bounds checks. Safe full-slice writes are available through
/// [`Output::write`] and complete writes through [`Output::write_fully`].
///
/// # Coherence note
///
/// Every [`std::io::Write`] value automatically implements `Output<Item = u8>`
/// through the standard I/O integration. Because [`Output::Item`] is an
/// associated type rather than a trait parameter, a concrete type that
/// implements [`std::io::Write`] cannot also provide any other direct `Output`
/// implementation for the same type, including one with a different item type.
///
/// Use a wrapper/newtype when a type needs item-oriented output semantics that
/// differ from its byte-oriented [`std::io::Write`] implementation.
///
/// # Method name overlap
///
/// `Output::write` has the same method name as [`std::io::Write::write`]
/// because both perform a safe single write for their respective abstraction
/// layer. In generic code where both traits are in scope for the same value,
/// use fully qualified syntax to choose the intended operation:
///
/// ```
/// use std::io::{
///     Result,
///     Write,
/// };
///
/// use qubit_io::Output;
///
/// fn flush_buffered<T>(output: &mut T) -> Result<()>
/// where
///     T: Output + Write,
/// {
///     <T as Output>::flush(output)
/// }
///
/// fn write_all_items<T>(output: &mut T, input: &[u8]) -> Result<()>
/// where
///     T: Output<Item = u8> + Write,
/// {
///     unsafe { <T as Output>::write_fully_unchecked(output, input, 0, input.len()) }
/// }
///
/// fn write_output_items<T>(output: &mut T, input: &[u8]) -> Result<usize>
/// where
///     T: Output<Item = u8> + Write,
/// {
///     Output::write(output, input)
/// }
///
/// fn flush_bytes<T>(output: &mut T) -> Result<()>
/// where
///     T: Output + Write,
/// {
///     Write::flush(output)
/// }
/// ```
pub trait Output {
    /// The item type written to this output.
    type Item;

    /// Returns whether this output already buffers items internally.
    ///
    /// # Returns
    ///
    /// `true` when callers should avoid wrapping this output in another generic
    /// item buffer automatically.
    #[inline(always)]
    #[must_use]
    fn is_buffered(&self) -> bool {
        false
    }

    /// Writes items from an indexed input range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Maximum number of items to write.
    ///
    /// # Returns
    ///
    /// The number of items accepted from `input[index..index + count]`. The
    /// value must be in `0..=count`.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Writes items from the full input slice.
    ///
    /// This method performs at most one write operation and keeps the same
    /// short-write behavior as [`Output::write_unchecked`].
    ///
    /// # Parameters
    ///
    /// * `input` - Source items.
    ///
    /// # Returns
    ///
    /// The number of items accepted from `input`.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::InvalidData`] if the implementation reports accepting more
    /// items than requested.
    #[inline(always)]
    fn write(&mut self, input: &[Self::Item]) -> Result<usize> {
        // SAFETY: The full input slice is a valid source range.
        let written = unsafe { self.write_unchecked(input, 0, input.len()) }?;
        validate_write_count(written, input.len())?;
        Ok(written)
    }

    /// Writes all items from an indexed input range.
    ///
    /// This method repeatedly calls [`Output::write_unchecked`] until all
    /// `count` items are accepted. Interrupted writes are retried. A zero
    /// progress report before the range is complete is converted to
    /// [`ErrorKind::WriteZero`].
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Number of items to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the requested range has been written completely.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::WriteZero`] if the implementation accepts zero items before
    /// the requested range is complete. Returns [`ErrorKind::InvalidData`] if
    /// the implementation reports accepting more items than requested.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested input range does not fit.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn write_fully_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<()> {
        debug_assert!(
            SliceRange::range_fits(input.len(), index, count),
            "unchecked write-fully range exceeds input buffer"
        );
        let mut written = 0;
        while written < count {
            let remaining = count - written;
            // SAFETY: The caller guarantees the original source range is valid;
            // `written < count`, so this suffix remains inside it.
            match unsafe {
                self.write_unchecked(input, index + written, remaining)
            } {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole output range",
                    ));
                }
                Ok(progress) => {
                    validate_write_count(progress, remaining)?;
                    written += progress;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Writes all items from the full input slice.
    ///
    /// # Parameters
    /// - `input`: Source items.
    ///
    /// # Returns
    /// `Ok(())` after every item in `input` has been written.
    ///
    /// # Errors
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::WriteZero`] if no progress is made before all items are
    /// accepted, or [`ErrorKind::InvalidData`] for impossible reported counts.
    #[inline(always)]
    fn write_fully(&mut self, input: &[Self::Item]) -> Result<()> {
        // SAFETY: The full input slice is a valid source range.
        unsafe { self.write_fully_unchecked(input, 0, input.len()) }
    }

    /// Flushes any internally buffered items.
    ///
    /// # Returns
    ///
    /// `Ok(())` after all internally buffered items have been flushed.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    fn flush(&mut self) -> Result<()>;
}
