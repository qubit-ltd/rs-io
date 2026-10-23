// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Adapter from a Qubit synchronous input to the standard library reader.

use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

use crate::ByteInput;
use crate::Input;

/// Adapts an [`Input`] of bytes to [`std::io::Read`].
///
/// Vectored reads forward the inner [`Input::read_vectored`] override when
/// buffers are non-empty; actual scatter/gather behavior depends on that
/// implementation and the platform. Constructing the adapter does not
/// allocate, but a vectored call may allocate temporary descriptors without
/// copying the payload. Length overflow returns `InvalidInput` and descriptor
/// allocation failure returns `OutOfMemory` before the inner call. A count
/// larger than the total capacity returns `InvalidData` after the inner call;
/// its side effects are not undone. Short progress and inner errors are
/// returned as reported, and interrupted reads are not retried.
///
/// A wrapped tee reports successful primary progress even if its mirror branch
/// fails. Check the tee's branch error after finite reads; a later non-empty
/// read returns the latched error.
///
/// # Type Parameters
///
/// - `I`: Input that supplies bytes to the standard reader.
///
/// # Examples
///
/// ```
/// use qubit_io::StdRead;
/// use std::io::Read;
///
/// let input = std::io::Cursor::new(b"hello".to_vec());
/// let mut reader = StdRead::new(input);
/// let mut bytes = Vec::new();
/// reader.read_to_end(&mut bytes).expect("reading should succeed");
/// assert_eq!(bytes, b"hello");
/// ```
#[must_use]
#[repr(transparent)]
pub struct StdRead<I> {
    /// Wrapped byte input.
    inner: I,
}

impl<I> StdRead<I> {
    /// Creates a standard reader around `inner`.
    #[inline]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped input.
    #[inline]
    pub const fn get_ref(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    #[inline]
    pub fn get_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Unwraps and returns the wrapped input.
    #[inline]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> Read for StdRead<I>
where
    I: ByteInput,
{
    #[inline]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        Input::read(&mut self.inner, output)
    }

    /// Reads into the supplied buffers in order, forwarding the inner
    /// vectored override. Descriptor preparation can fail before the inner
    /// call; an invalid count is detected after that call.
    #[inline]
    fn read_vectored(&mut self, buffers: &mut [std::io::IoSliceMut<'_>]) -> Result<usize> {
        let capacity = crate::util::requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let mut slices = crate::util::byte_slices_mut(buffers)?;
        let count = Input::read_vectored(&mut self.inner, &mut slices)?;
        crate::traits::validate_read_count(count, capacity)?;
        Ok(count)
    }
}

impl<I> Seek for StdRead<I>
where
    I: ByteInput + crate::Seekable<Unit = u8>,
{
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        crate::Seekable::seek_to(&mut self.inner, position)
    }
}
