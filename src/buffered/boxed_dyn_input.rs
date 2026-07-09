// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Result;

use crate::Input;

/// Concrete input wrapper around a boxed input trait object.
pub(crate) struct BoxedDynInput<'a, T> {
    input: Box<dyn Input<Item = T> + 'a>,
}

impl<'a, T> BoxedDynInput<'a, T> {
    /// Creates a concrete wrapper around a boxed input trait object.
    ///
    /// # Parameters
    ///
    /// * `input` - The boxed input trait object to wrap.
    ///
    /// # Returns
    ///
    /// A concrete input wrapper that forwards reads to `input`.
    #[inline(always)]
    pub(crate) const fn new(input: Box<dyn Input<Item = T> + 'a>) -> Self {
        Self { input }
    }
}

impl<T> Input for BoxedDynInput<'_, T> {
    type Item = T;

    /// Reads items through the wrapped input trait object.
    #[inline(always)]
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe { self.input.read_unchecked(output, index, count) }
    }
}
