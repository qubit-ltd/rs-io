/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use super::CoderStatus;

/// Counts how much work a [`crate::Coder`] completed before returning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoderProgress {
    /// Stop reason reported by the coder.
    status: CoderStatus,
    /// Number of input units consumed from the requested input index.
    read: usize,
    /// Number of output units written from the requested output index.
    written: usize,
}

impl CoderProgress {
    /// Creates a progress value.
    ///
    /// # Parameters
    ///
    /// - `status`: The reason conversion stopped.
    /// - `read`: Number of input units consumed from the call's input index.
    /// - `written`: Number of output units written from the call's output index.
    ///
    /// # Returns
    ///
    /// Returns a progress value carrying the supplied counters.
    #[must_use]
    #[inline]
    pub const fn new(status: CoderStatus, read: usize, written: usize) -> Self {
        Self { status, read, written }
    }

    /// Creates a completed progress value.
    ///
    /// # Parameters
    ///
    /// - `read`: Number of consumed input units.
    /// - `written`: Number of produced output units.
    ///
    /// # Returns
    ///
    /// Returns a progress value whose status is [`CoderStatus::Complete`].
    #[must_use]
    #[inline]
    pub const fn complete(read: usize, written: usize) -> Self {
        Self::new(CoderStatus::Complete, read, written)
    }

    /// Returns the status that stopped conversion.
    ///
    /// # Returns
    ///
    /// Returns the stored [`CoderStatus`].
    #[must_use]
    #[inline]
    pub const fn status(self) -> CoderStatus {
        self.status
    }

    /// Returns the number of input units consumed by the call.
    ///
    /// # Returns
    ///
    /// Returns a count relative to the input index passed to the conversion call.
    #[must_use]
    #[inline]
    pub const fn read(self) -> usize {
        self.read
    }

    /// Returns the number of output units written by the call.
    ///
    /// # Returns
    ///
    /// Returns a count relative to the output index passed to the conversion call.
    #[must_use]
    #[inline]
    pub const fn written(self) -> usize {
        self.written
    }

    /// Returns the additional unit count required by the reported status.
    ///
    /// # Returns
    ///
    /// Returns `0` when conversion completed.
    #[must_use]
    #[inline]
    pub const fn required(self) -> usize {
        match self.status {
            CoderStatus::Complete => 0,
            CoderStatus::NeedInput { required, .. } => required,
            CoderStatus::NeedOutput { required, .. } => required,
        }
    }

    /// Returns the absolute boundary index associated with this status, if any.
    ///
    /// - For [`CoderStatus::NeedInput`], returns `input_index`.
    /// - For [`CoderStatus::NeedOutput`], returns `output_index`.
    /// - For [`CoderStatus::Complete`], returns `None`.
    #[must_use]
    #[inline]
    pub const fn index(self) -> Option<usize> {
        match self.status {
            CoderStatus::Complete => None,
            CoderStatus::NeedInput { input_index, .. } => Some(input_index),
            CoderStatus::NeedOutput { output_index, .. } => Some(output_index),
        }
    }

    /// Returns the number of available units at the reported status boundary.
    ///
    /// # Returns
    ///
    /// Returns `0` when conversion completed.
    #[must_use]
    #[inline]
    pub const fn available(self) -> usize {
        match self.status {
            CoderStatus::Complete => 0,
            CoderStatus::NeedInput { available, .. } => available,
            CoderStatus::NeedOutput { available, .. } => available,
        }
    }
}
