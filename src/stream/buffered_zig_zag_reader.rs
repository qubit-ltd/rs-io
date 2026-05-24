/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use core::marker::PhantomData;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Seek,
    SeekFrom,
};

use crate::codec::{
    DecodePolicy,
    Leb128DecodeError,
    NonStrict,
    Strict,
    ZigZagCodec,
};
use crate::stream::BufferedInput;

/// Buffered reader for ZigZag + unsigned LEB128 integers.
///
/// Values are decoded directly from the internal input buffer while the codec
/// scans for the underlying LEB128 terminating byte.
///
/// # Buffered state
///
/// This reader may prefetch bytes from the wrapped reader. As a result,
/// [`Self::get_ref`] can observe an underlying stream position ahead of the
/// logical position exposed by this wrapper, and [`Self::into_inner`] discards
/// any prefetched bytes that have not been consumed.
///
/// # Target-width integers
///
/// `isize` methods use the current Rust target's pointer width. Prefer
/// fixed-width integer methods such as `read_i64` for persistent files and
/// cross-platform protocols.
pub struct BufferedZigZagReader<R, P = NonStrict> {
    input: BufferedInput<R>,
    marker: PhantomData<fn() -> P>,
}

impl<R, P> BufferedZigZagReader<R, P>
where
    P: DecodePolicy,
{
    /// Creates a buffered ZigZag reader with the default buffer capacity.
    #[must_use]
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            input: BufferedInput::new(inner),
            marker: PhantomData,
        }
    }

    /// Creates a buffered ZigZag reader with at least `capacity` bytes.
    #[must_use]
    #[inline]
    pub fn with_capacity(inner: R, capacity: usize) -> Self {
        Self {
            input: BufferedInput::with_capacity(inner, capacity),
            marker: PhantomData,
        }
    }

    /// Returns whether this reader rejects non-canonical LEB128 encodings.
    #[must_use]
    #[inline]
    pub const fn is_strict(&self) -> bool {
        P::STRICT
    }

    /// Returns a shared reference to the underlying reader.
    ///
    /// The underlying reader may already be positioned past unread bytes held
    /// in this wrapper's internal buffer.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &R {
        self.input.get_ref()
    }

    /// Returns an exclusive reference to the underlying reader.
    ///
    /// Mutating the underlying reader directly can invalidate prefetched bytes
    /// already held in this wrapper's internal buffer.
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        self.input.get_mut()
    }

    /// Consumes this wrapper and returns the underlying reader.
    ///
    /// Any bytes already prefetched into the internal buffer but not consumed
    /// by codec methods are discarded.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> R {
        self.input.into_inner()
    }
}

macro_rules! impl_read_value {
    ($policy:ty, $method:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[inline]
        pub fn $method(&mut self) -> Result<$ty> {
            type Codec = ZigZagCodec<$ty, $policy>;

            self.input
                .read_variable_decoded::<{ Codec::REQUIRED_MIN_BUFFER_LEN }, _, _, _, _>(
                    |bytes, index, available| {
                        // SAFETY: `read_variable_decoded` only passes bytes already
                        // present in the internal buffer and caps `available` at
                        // the codec maximum width.
                        unsafe { Codec::read_available_unchecked(bytes, index, available) }
                    },
                    map_leb128_decode_error,
                )
        }
    };
}

macro_rules! impl_for_policy {
    ($policy:ty) => {
        impl<R> BufferedZigZagReader<R, $policy>
        where
            R: Read,
        {
            impl_read_value!($policy, read_i8, i8, "Reads a ZigZag `i8`.");
            impl_read_value!($policy, read_i16, i16, "Reads a ZigZag `i16`.");
            impl_read_value!($policy, read_i32, i32, "Reads a ZigZag `i32`.");
            impl_read_value!($policy, read_i64, i64, "Reads a ZigZag `i64`.");
            impl_read_value!($policy, read_i128, i128, "Reads a ZigZag `i128`.");
            impl_read_value!($policy, read_isize, isize, "Reads a ZigZag `isize`.");
        }
    };
}

impl_for_policy!(NonStrict);
impl_for_policy!(Strict);

impl<R, P> Read for BufferedZigZagReader<R, P>
where
    R: Read,
{
    /// Reads bytes from the buffered reader.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.input.read_raw(buffer)
    }
}

impl<R, P> Seek for BufferedZigZagReader<R, P>
where
    R: Read + Seek,
{
    /// Seeks the wrapped reader and discards buffered bytes after success.
    #[inline]
    fn seek(&mut self, position: SeekFrom) -> Result<u64> {
        self.input.seek_raw(position)
    }
}

/// Converts a LEB128 decode error into an I/O error.
#[inline]
fn map_leb128_decode_error(error: Leb128DecodeError) -> Error {
    Error::new(ErrorKind::InvalidData, error)
}
