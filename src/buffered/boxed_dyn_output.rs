// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Result;

use crate::Output;

/// Concrete output wrapper around a boxed output trait object.
pub(crate) struct BoxedDynOutput<'a, T> {
    output: Box<dyn Output<Item = T> + 'a>,
}

impl<'a, T> BoxedDynOutput<'a, T> {
    /// Creates a concrete wrapper around a boxed output trait object.
    ///
    /// # Parameters
    ///
    /// * `output` - The boxed output trait object to wrap.
    ///
    /// # Returns
    ///
    /// A concrete output wrapper that forwards writes to `output`.
    #[inline(always)]
    pub(crate) const fn new(output: Box<dyn Output<Item = T> + 'a>) -> Self {
        Self { output }
    }
}

impl<T> Output for BoxedDynOutput<'_, T> {
    type Item = T;

    /// Writes items through the wrapped output trait object.
    #[inline(always)]
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> Result<usize> {
        // SAFETY: Forwarded from the trait caller.
        unsafe { self.output.write_unchecked(input, index, count) }
    }

    /// Flushes the wrapped output trait object.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        self.output.flush()
    }
}
