// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    Result,
    Write,
};

/// Minimal indexed output interface over units.
///
/// `Output` is intentionally smaller and lower-level than [`Write`]. It only
/// states that an implementor can write up to `count` units from
/// `input[index..index + count]`, plus an explicit flush operation. The caller
/// owns range validation so hot paths can avoid repeated slicing and bounds
/// checks.
pub trait Output {
    /// The unit type written to this output.
    type Item;

    /// Writes units from an indexed input range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Maximum number of units to write.
    ///
    /// # Returns
    ///
    /// The number of units accepted from `input[index..index + count]`. The
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

    /// Flushes any internally buffered units.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    fn flush(&mut self) -> Result<()>;
}

impl<W> Output for W
where
    W: Write + ?Sized,
{
    type Item = u8;

    /// Writes bytes to a standard [`Write`] value from an indexed range.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        debug_assert!(
            index
                .checked_add(count)
                .is_some_and(|end| end <= input.len()),
            "unchecked write range exceeds input buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `input`.
        let source = unsafe {
            core::slice::from_raw_parts(input.as_ptr().add(index), count)
        };
        self.write(source)
    }

    /// Flushes a standard [`Write`] value.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self)
    }
}
