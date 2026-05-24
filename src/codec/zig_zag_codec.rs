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

use crate::{
    DecodePolicy,
    Leb128Codec,
    Leb128DecodeError,
    NonStrict,
};

/// Type-level unchecked ZigZag + unsigned LEB128 codec.
///
/// `T` selects the signed integer type and `P` selects the LEB128 decoding
/// policy used after ZigZag conversion.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZigZagCodec<T, P = NonStrict> {
    marker: PhantomData<fn() -> (T, P)>,
}

macro_rules! impl_zig_zag_codec {
    ($signed:ty, $unsigned:ty, $shift:expr) => {
        impl<P> ZigZagCodec<$signed, P>
        where
            P: DecodePolicy,
        {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = Leb128Codec::<$unsigned, NonStrict>::REQUIRED_MIN_BUFFER_LEN;

            /// Decodes a value from `input` starting at `index` without bounds checks.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start index in `input`.
            ///
            /// # Returns
            ///
            /// Returns the decoded value and the number of consumed bytes.
            ///
            /// # Errors
            ///
            /// Returns [`Leb128DecodeError`] if the underlying LEB128 bytes are invalid.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `input.as_ptr().add(index)` is valid to
            /// read [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes, or that a valid terminating byte
            /// appears before that limit.
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> Result<($signed, usize), Leb128DecodeError> {
                // SAFETY: The caller guarantees enough readable bytes for this type.
                let (encoded, consumed) = unsafe { Leb128Codec::<$unsigned, P>::read_unchecked(input, index)? };
                let value = ((encoded >> 1) as $signed) ^ (-((encoded & 1) as $signed));
                Ok((value, consumed))
            }

            /// Tries to decode from the currently available bytes without bounds checks.
            ///
            /// This internal entry point lets buffered readers decode the underlying
            /// unsigned LEB128 payload while scanning for its terminating byte.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start index in `input`.
            /// - `available`: Number of readable bytes currently available from
            ///   `index`.
            ///
            /// # Returns
            ///
            /// Returns `Ok(Some((value, consumed)))` when a complete value is
            /// decoded. Returns `Ok(None)` when more bytes are needed. Returns
            /// `Err((error, consumed))` when the underlying LEB128 payload is
            /// invalid and should be consumed before the error is reported.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `input.as_ptr().add(index)` is valid
            /// to read `available` bytes and that `available` is no greater than
            /// [`Self::REQUIRED_MIN_BUFFER_LEN`].
            #[inline(always)]
            pub(crate) unsafe fn read_available_unchecked(
                input: &[u8],
                index: usize,
                available: usize,
            ) -> Result<Option<($signed, usize)>, (Leb128DecodeError, usize)> {
                // SAFETY: The caller guarantees that exactly `available` bytes
                // are readable from `index`.
                let result = unsafe { Leb128Codec::<$unsigned, P>::read_available_unchecked(input, index, available)? };
                Ok(result.map(|(encoded, consumed)| {
                    let value = ((encoded >> 1) as $signed) ^ (-((encoded & 1) as $signed));
                    (value, consumed)
                }))
            }

            /// Encodes `value` into `output` starting at `index` without bounds checks.
            ///
            /// # Parameters
            ///
            /// - `output`: Destination byte buffer.
            /// - `index`: Start index in `output`.
            /// - `value`: Value to encode.
            ///
            /// # Returns
            ///
            /// Returns the number of written bytes.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `output.as_mut_ptr().add(index)` is valid
            /// to write [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes.
            #[inline(always)]
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $signed) -> usize {
                let encoded = ((value as $unsigned) << 1) ^ ((value >> $shift) as $unsigned);
                // SAFETY: The caller guarantees enough writable bytes for this type.
                unsafe { Leb128Codec::<$unsigned, NonStrict>::write_unchecked(output, index, encoded) }
            }
        }
    };
}

impl_zig_zag_codec!(i8, u8, 7);
impl_zig_zag_codec!(i16, u16, 15);
impl_zig_zag_codec!(i32, u32, 31);
impl_zig_zag_codec!(i64, u64, 63);
impl_zig_zag_codec!(i128, u128, 127);
impl_zig_zag_codec!(isize, usize, isize::BITS - 1);
