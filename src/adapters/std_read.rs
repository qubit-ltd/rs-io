// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Adapter from a Qubit synchronous input to the standard library reader.

use std::io::IoSliceMut;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

use crate::ByteInput;
use crate::Input;
use crate::Seekable;
use crate::traits::validate_read_count;
use crate::util::byte_slices_mut;
use crate::util::requested_items;

/// Adapts an [`Input`] of bytes to [`std::io::Read`].
///
/// Vectored reads forward the inner [`Input::read_vectored`] override when
/// buffers are non-empty; actual scatter/gather behavior depends on that
/// implementation and the platform. Constructing the adapter does not
/// allocate. Exactly one descriptor uses stack storage; multiple descriptors
/// may allocate temporary descriptor storage, and the inner input may allocate
/// independently. Length overflow returns `InvalidInput`; multiple-descriptor
/// preparation failure returns `OutOfMemory` before the inner call. A count
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
///
/// A read-only input implements `Read` without providing seek capability:
///
/// ```
/// use qubit_io::{Input, StdRead};
/// use std::io::{Read, Result};
///
/// struct ReadOnly;
/// impl Input for ReadOnly {
///     type Item = u8;
///     unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
///         Ok(0)
///     }
/// }
/// fn require_read<T: Read>(_: &T) {}
/// require_read(&StdRead::new(ReadOnly));
/// ```
///
/// It does not provide `Seek` unless its input is seekable:
///
/// ```compile_fail,E0277
/// use qubit_io::{Input, StdRead};
/// use std::io::{Result, Seek};
///
/// struct ReadOnly;
/// impl Input for ReadOnly {
///     type Item = u8;
///     unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
///         Ok(0)
///     }
/// }
/// fn require_seek<T: Seek>() {}
/// require_seek::<StdRead<ReadOnly>>();
/// ```
#[must_use]
#[repr(transparent)]
pub struct StdRead<I> {
    /// Wrapped byte input.
    inner: I,
}

impl<I> StdRead<I> {
    /// Creates a standard reader around `inner`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Byte input that supplies data to the standard reader.
    ///
    /// # Returns
    ///
    /// Returns an adapter that owns `inner`.
    #[inline]
    pub const fn new(inner: I) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input borrowed for the lifetime of `self`.
    #[must_use = "the wrapped input reference should be used"]
    #[inline]
    pub const fn get_ref(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input mutably borrowed for the lifetime of `self`.
    #[must_use = "the wrapped input reference should be used"]
    #[inline]
    pub fn get_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Unwraps and returns the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the owned input, consuming the adapter.
    #[inline]
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> Read for StdRead<I>
where
    I: ByteInput,
{
    /// Reads bytes from the wrapped input into `output`.
    ///
    /// # Parameters
    ///
    /// - `output`: Buffer that receives bytes from the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read. A zero count means `output` is empty
    /// or the input reached EOF.
    ///
    /// # Errors
    ///
    /// Returns the wrapped input's error unchanged. Returns `InvalidData` if
    /// the wrapped input reports more bytes than `output` can hold. Validation
    /// does not undo side effects of the inner call.
    #[inline]
    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        let read = Input::read(&mut self.inner, output)?;
        validate_read_count(read, output.len())?;
        Ok(read)
    }

    /// Reads into the supplied buffers in order, forwarding the inner
    /// vectored override.
    ///
    /// # Parameters
    ///
    /// - `buffers`: Mutable buffers filled in order by the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read. A zero count means every buffer has
    /// zero capacity or the input reached EOF. Zero-capacity buffers return
    /// zero without calling the wrapped input.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` if the requested capacity overflows, or
    /// `OutOfMemory` if multiple-descriptor preparation cannot allocate.
    /// Returns the wrapped input's error unchanged. Returns `InvalidData` if
    /// the wrapped input reports more bytes than the buffers can hold; its
    /// side effects are not undone.
    #[inline]
    fn read_vectored(&mut self, buffers: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let capacity = requested_items(buffers.iter().map(|buffer| buffer.len()))?;
        if capacity == 0 {
            return Ok(0);
        }
        let count = if let [buffer] = buffers {
            let mut slices = [&mut buffer[..]];
            Input::read_vectored(&mut self.inner, &mut slices)?
        } else {
            let mut slices = byte_slices_mut(buffers)?;
            Input::read_vectored(&mut self.inner, &mut slices)?
        };
        validate_read_count(count, capacity)?;
        Ok(count)
    }
}

impl<I> Seek for StdRead<I>
where
    I: ByteInput + Seekable<Unit = u8>,
{
    /// Seeks the wrapped input to a byte position.
    ///
    /// # Parameters
    ///
    /// - `position`: Target byte position or offset.
    ///
    /// # Returns
    ///
    /// Returns the new absolute byte position.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the wrapped input.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        Seekable::seek_to(&mut self.inner, position)
    }
}
