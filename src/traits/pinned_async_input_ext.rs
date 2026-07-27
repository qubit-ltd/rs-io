// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::{
    AsyncInput,
    ReadExactFuture,
    ReadFullyFuture,
    ReadFuture,
};

/// Convenience futures for an already pinned asynchronous input.
pub trait PinnedAsyncInputExt {
    /// The pinned asynchronous input type.
    type Input: AsyncInput + ?Sized;

    /// Creates a future that performs one read.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of items read.
    fn read_async<'a>(
        &'a mut self,
        output: &'a mut [<Self::Input as AsyncInput>::Item],
    ) -> ReadFuture<'a, Self::Input>;

    /// Creates a future that reads until the destination is full or EOF.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the total number of items read.
    fn read_fully_async<'a>(
        &'a mut self,
        output: &'a mut [<Self::Input as AsyncInput>::Item],
    ) -> ReadFullyFuture<'a, Self::Input>;

    /// Creates a future that fills the entire destination.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage that must be filled completely.
    ///
    /// # Returns
    ///
    /// A future that resolves after filling `output` or encountering an error.
    fn read_exactly_async<'a>(
        &'a mut self,
        output: &'a mut [<Self::Input as AsyncInput>::Item],
    ) -> ReadExactFuture<'a, Self::Input>;
}

impl<I> PinnedAsyncInputExt for Pin<&mut I>
where
    I: AsyncInput + ?Sized,
{
    /// The asynchronous input behind this pinned reference.
    type Input = I;

    /// Creates a single-read future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
        output: &'a mut [I::Item],
    ) -> ReadFuture<'a, Self::Input> {
        ReadFuture::new(self.as_mut(), output)
    }

    /// Creates a read-fully future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
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
        output: &'a mut [I::Item],
    ) -> ReadFullyFuture<'a, Self::Input> {
        ReadFullyFuture::new(self.as_mut(), output)
    }

    /// Creates an exact-read future for this pinned input.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned input borrow and destination.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage that must be filled completely.
    ///
    /// # Returns
    ///
    /// A future that resolves after filling `output` or encountering an error.
    #[inline(always)]
    fn read_exactly_async<'a>(
        &'a mut self,
        output: &'a mut [I::Item],
    ) -> ReadExactFuture<'a, Self::Input> {
        ReadExactFuture::new(self.as_mut(), output)
    }
}
