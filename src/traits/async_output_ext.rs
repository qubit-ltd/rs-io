// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::pin::Pin;

use crate::{
    AsyncOutput,
    FlushFuture,
    WriteFullyFuture,
    WriteFuture,
};

/// Future-based convenience operations for [`AsyncOutput`].
pub trait AsyncOutputExt: AsyncOutput {
    /// Creates a future that performs one asynchronous write operation.
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
        input: &'a [Self::Item],
    ) -> WriteFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteFuture::new(Pin::new(self), input)
    }

    /// Creates a future that writes the entire source slice.
    ///
    /// The returned future retries interrupted operations and reports
    /// [`std::io::ErrorKind::WriteZero`] when output makes no progress.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    ///
    /// # Returns
    ///
    /// A future that resolves when every item has been accepted.
    #[inline(always)]
    fn write_fully_async<'a>(
        &'a mut self,
        input: &'a [Self::Item],
    ) -> WriteFullyFuture<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteFullyFuture::new(Pin::new(self), input)
    }

    /// Creates a future that flushes internally buffered items.
    ///
    /// # Returns
    ///
    /// A future that resolves with the flush result.
    #[inline(always)]
    fn flush_async(&mut self) -> FlushFuture<'_, Self>
    where
        Self: Sized + Unpin,
    {
        FlushFuture::new(Pin::new(self))
    }
}

impl<O> AsyncOutputExt for O where O: AsyncOutput + ?Sized {}
