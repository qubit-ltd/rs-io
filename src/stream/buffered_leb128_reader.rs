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
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};

use crate::codec::{DecodePolicy, Leb128Codec, Leb128DecodeError, NonStrict, Strict};
use crate::stream::BufferedInput;
use crate::util::read_utf8_payload;

/// Buffered reader for LEB128 integers.
///
/// Values are decoded directly from the internal input buffer once a terminating
/// byte or the codec maximum width is available.
pub struct BufferedLeb128Reader<R, P = NonStrict> {
    input: BufferedInput<R>,
    marker: PhantomData<fn() -> P>,
}

impl<R, P> BufferedLeb128Reader<R, P>
where
    P: DecodePolicy,
{
    /// Creates a buffered LEB128 reader with the default buffer capacity.
    #[must_use]
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            input: BufferedInput::new(inner),
            marker: PhantomData,
        }
    }

    /// Creates a buffered LEB128 reader with at least `capacity` bytes.
    #[must_use]
    #[inline]
    pub fn with_capacity(inner: R, capacity: usize) -> Self {
        Self {
            input: BufferedInput::with_capacity(inner, capacity),
            marker: PhantomData,
        }
    }

    /// Returns whether this reader rejects non-canonical encodings.
    #[must_use]
    #[inline]
    pub const fn is_strict(&self) -> bool {
        P::STRICT
    }

    /// Returns a shared reference to the underlying reader.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &R {
        self.input.get_ref()
    }

    /// Returns an exclusive reference to the underlying reader.
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
            type Codec = Leb128Codec<$ty, $policy>;

            self.input.read_variable(
                Codec::REQUIRED_MIN_BUFFER_LEN,
                |bytes, index| {
                    // SAFETY: `read_variable` guarantees that either a
                    // terminating byte or the codec maximum width is available.
                    unsafe { Codec::read_unchecked(bytes, index) }
                },
                map_leb128_decode_error,
            )
        }
    };
}

macro_rules! impl_for_policy {
    ($policy:ty) => {
        impl<R> BufferedLeb128Reader<R, $policy>
        where
            R: Read,
        {
            impl_read_value!($policy, read_u8, u8, "Reads an unsigned LEB128 `u8`.");
            impl_read_value!($policy, read_u16, u16, "Reads an unsigned LEB128 `u16`.");
            impl_read_value!($policy, read_u32, u32, "Reads an unsigned LEB128 `u32`.");
            impl_read_value!($policy, read_u64, u64, "Reads an unsigned LEB128 `u64`.");
            impl_read_value!($policy, read_u128, u128, "Reads an unsigned LEB128 `u128`.");
            impl_read_value!(
                $policy,
                read_usize,
                usize,
                "Reads an unsigned LEB128 `usize`."
            );
            impl_read_value!($policy, read_i8, i8, "Reads a signed LEB128 `i8`.");
            impl_read_value!($policy, read_i16, i16, "Reads a signed LEB128 `i16`.");
            impl_read_value!($policy, read_i32, i32, "Reads a signed LEB128 `i32`.");
            impl_read_value!($policy, read_i64, i64, "Reads a signed LEB128 `i64`.");
            impl_read_value!($policy, read_i128, i128, "Reads a signed LEB128 `i128`.");
            impl_read_value!($policy, read_isize, isize, "Reads a signed LEB128 `isize`.");

            /// Reads a UTF-8 string prefixed by an unsigned LEB128 byte length.
            #[inline]
            pub fn read_utf8_string(&mut self, max_len: usize) -> Result<String> {
                let len = self.read_usize()?;
                read_utf8_payload(&mut self.input, len, max_len)
            }
        }
    };
}

impl_for_policy!(NonStrict);
impl_for_policy!(Strict);

impl<R, P> Read for BufferedLeb128Reader<R, P>
where
    R: Read,
{
    /// Reads bytes from the buffered reader.
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.input.read_raw(buffer)
    }
}

impl<R, P> Seek for BufferedLeb128Reader<R, P>
where
    R: Read + Seek,
{
    /// Seeks the wrapped reader after discarding buffered bytes.
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
