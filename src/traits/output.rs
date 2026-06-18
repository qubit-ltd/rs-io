// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Result, Write};

use crate::util::UncheckedSlice;

/// Minimal indexed output interface over items.
///
/// `Output` is intentionally smaller and lower-level than [`Write`]. It only
/// states that an implementor can write up to `count` items from
/// `input[index..index + count]`, plus an explicit flush operation. The caller
/// owns range validation so hot paths can avoid repeated slicing and bounds
/// checks.
///
/// # Method name overlap
///
/// `Output::write_from` and `Output::flush_pending` names are intentionally
/// distinct from [`Write::write`] and [`Write::flush`]. In generic code where
/// both traits are in scope for the same value, use fully qualified syntax to
/// choose the intended operation:
///
/// ```
/// use std::io::{
///     Result,
///     Write,
/// };
///
/// use qubit_io::{
///     Output,
///     OutputExt,
/// };
///
/// fn flush_buffered<T>(output: &mut T) -> Result<()>
/// where
///     T: Output + Write,
/// {
///     <T as Output>::flush_pending(output)
/// }
///
/// fn write_all_items<T>(output: &mut T, input: &[u8]) -> Result<()>
/// where
///     T: Output<Item = u8> + Write,
/// {
///     unsafe { <T as OutputExt>::write_all_from(output, input, 0, input.len()) }
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
    /// The unit type written to this output.
    type Item;

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
    unsafe fn write_from(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize>;

    /// Flushes any internally buffered items.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    fn flush_pending(&mut self) -> Result<()>;
}

impl<W> Output for W
where
    W: Write + ?Sized,
{
    type Item = u8;

    /// Writes bytes to a standard [`Write`] value from an indexed range.
    #[inline(always)]
    unsafe fn write_from(&mut self, input: &[u8], index: usize, count: usize) -> Result<usize> {
        debug_assert!(
            UncheckedSlice::range_fits(input.len(), index, count),
            "unchecked write range exceeds input buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `input`.
        let source = unsafe { UncheckedSlice::subslice(input, index, count) };
        Write::write(self, source)
    }

    /// Flushes a standard [`Write`] value.
    #[inline(always)]
    fn flush_pending(&mut self) -> Result<()> {
        Write::flush(self)
    }
}
