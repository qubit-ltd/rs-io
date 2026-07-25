// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::{
    AsyncClose,
    AsyncOutput,
    CloseFuture,
    FlushFuture,
    WriteFullyFuture,
    WriteFuture,
};

/// Convenience futures for an already pinned asynchronous output.
pub trait PinnedAsyncOutputExt {
    /// The pinned asynchronous output type.
    type Output: AsyncOutput + ?Sized;

    /// Creates a future that performs one write.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of accepted items.
    fn write_async<'a>(
        &'a mut self,
        input: &'a [<Self::Output as AsyncOutput>::Item],
    ) -> WriteFuture<'a, Self::Output>;

    /// Creates a future that writes the entire source.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves after every item has been accepted.
    fn write_fully_async<'a>(
        &'a mut self,
        input: &'a [<Self::Output as AsyncOutput>::Item],
    ) -> WriteFullyFuture<'a, Self::Output>;

    /// Creates a future that flushes the output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    fn flush_async(&mut self) -> FlushFuture<'_, Self::Output>;

    /// Creates a future that closes the output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the close result.
    fn close_async(&mut self) -> CloseFuture<'_, Self::Output>
    where
        Self::Output: AsyncClose;
}

impl<O> PinnedAsyncOutputExt for Pin<&mut O>
where
    O: AsyncOutput + ?Sized,
{
    /// The asynchronous output behind this pinned reference.
    type Output = O;

    /// Creates a single-write future for this pinned output.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves with the number of accepted items.
    #[inline(always)]
    fn write_async<'a>(
        &'a mut self,
        input: &'a [O::Item],
    ) -> WriteFuture<'a, Self::Output> {
        WriteFuture::new(self.as_mut(), input)
    }

    /// Creates a write-fully future for this pinned output.
    ///
    /// # Type Parameters
    ///
    /// * `'a` - Shared lifetime of the pinned output borrow and source.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves after every item has been accepted.
    #[inline(always)]
    fn write_fully_async<'a>(
        &'a mut self,
        input: &'a [O::Item],
    ) -> WriteFullyFuture<'a, Self::Output> {
        WriteFullyFuture::new(self.as_mut(), input)
    }

    /// Creates a flush future for this pinned output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    #[inline(always)]
    fn flush_async(&mut self) -> FlushFuture<'_, Self::Output> {
        FlushFuture::new(self.as_mut())
    }

    /// Creates a close future for this pinned output.
    ///
    /// # Returns
    ///
    /// A future that resolves with the close result.
    #[inline(always)]
    fn close_async(&mut self) -> CloseFuture<'_, Self::Output>
    where
        Self::Output: AsyncClose,
    {
        CloseFuture::new(self.as_mut())
    }
}
