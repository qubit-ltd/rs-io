// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::{
    AsyncInput,
    ReadFullyFuture,
    ReadFuture,
};

/// Future-based convenience operations for [`AsyncInput`].
pub trait AsyncInputExt: AsyncInput {
    /// Creates a future that performs one asynchronous read operation.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of items read.
    #[inline(always)]
    fn read_async<'a>(
        &'a mut self,
        output: &'a mut [Self::Item],
    ) -> ReadFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFuture::new(Pin::new(self), output)
    }

    /// Creates a future that fills a destination as far as possible.
    ///
    /// The returned future retries interrupted operations and stops when the
    /// destination is full or the input reports EOF.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the total number of items read.
    #[inline(always)]
    fn read_fully_async<'a>(
        &'a mut self,
        output: &'a mut [Self::Item],
    ) -> ReadFullyFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadFullyFuture::new(Pin::new(self), output)
    }
}

impl<I> AsyncInputExt for I where I: AsyncInput + ?Sized {}
