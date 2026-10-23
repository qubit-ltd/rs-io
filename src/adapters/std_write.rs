// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Adapter from a Qubit synchronous output to the standard library writer.

use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use crate::ByteOutput;
use crate::Output;

/// Adapts an [`Output`] of bytes to [`std::io::Write`].
///
/// Vectored writes forward the inner [`Output::write_vectored`] override when
/// buffers are non-empty; actual scatter/gather behavior depends on that
/// implementation and the platform. Constructing the adapter does not
/// allocate, but a vectored call may allocate temporary descriptors without
/// copying the payload. Length overflow returns `InvalidInput` and descriptor
/// allocation failure returns `OutOfMemory` before the inner call. A count
/// larger than the total capacity returns `InvalidData` after the inner call;
/// its side effects are not undone. Short progress and inner errors are
/// returned as reported, and interrupted writes are not retried.
///
/// A wrapped tee reports successful primary progress even if its mirror branch
/// fails. Check `flush` after finite writes; a later non-empty write and flush
/// return the latched error.
///
/// # Type Parameters
///
/// - `O`: Output that accepts bytes.
///
/// # Examples
///
/// ```
/// use qubit_io::StdWrite;
/// use std::io::Write;
///
/// let mut writer = StdWrite::new(std::io::Cursor::new(Vec::new()));
/// writer.write_all(b"hello").expect("writing should succeed");
/// assert_eq!(writer.into_inner().into_inner(), b"hello");
/// ```
#[must_use]
#[repr(transparent)]
pub struct StdWrite<O> {
    /// Wrapped byte output.
    inner: O,
}

impl<O> StdWrite<O> {
    /// Creates a standard writer around `inner`.
    #[inline]
    pub const fn new(inner: O) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped output.
    #[inline]
    pub const fn get_ref(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    #[inline]
    pub fn get_mut(&mut self) -> &mut O {
        &mut self.inner
    }

    /// Unwraps and returns the wrapped output.
    #[inline]
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O> Write for StdWrite<O>
where
    O: ByteOutput,
{
    #[inline]
    fn write(&mut self, input: &[u8]) -> Result<usize> {
        Output::write(&mut self.inner, input)
    }

    /// Writes from the supplied buffers in order, forwarding the inner
    /// vectored override. Descriptor preparation can fail before the inner
    /// call; an invalid count is detected after that call.
    #[inline]
    fn write_vectored(&mut self, buffers: &[std::io::IoSlice<'_>]) -> Result<usize> {
        let capacity = crate::util::requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let slices = crate::util::byte_slices(buffers)?;
        let count = Output::write_vectored(&mut self.inner, &slices)?;
        crate::traits::validate_write_count(count, capacity)?;
        Ok(count)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Output::flush(&mut self.inner)
    }
}

impl<O> Seek for StdWrite<O>
where
    O: ByteOutput + crate::Seekable<Unit = u8>,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        crate::Seekable::seek_to(&mut self.inner, position)
    }
}
