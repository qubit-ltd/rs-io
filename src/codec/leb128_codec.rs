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
    Leb128DecodeError,
    Leb128DecodeErrorKind,
    NonStrict,
};

/// Type-level unchecked LEB128 codec.
///
/// `T` selects the decoded integer type and `P` selects the decoding policy.
/// Encoding is always canonical; `P` only affects decoding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Leb128Codec<T, P = NonStrict> {
    marker: PhantomData<fn() -> (T, P)>,
}

macro_rules! impl_unsigned_leb128_codec {
    ($ty:ty) => {
        impl<P> Leb128Codec<$ty, P>
        where
            P: DecodePolicy,
        {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = (<$ty>::BITS as usize).div_ceil(7);

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
            /// Returns [`Leb128DecodeError`] if the bytes are malformed or strict
            /// decoding rejects a non-canonical representation.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `input.as_ptr().add(index)` is valid to
            /// read [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes, or that a valid terminating byte
            /// appears before that limit.
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> Result<($ty, usize), Leb128DecodeError> {
                // SAFETY: The caller guarantees enough readable bytes for this type.
                let (value, consumed) =
                    unsafe { read_uleb_unchecked::<P>(input, index, <$ty>::BITS, Self::REQUIRED_MIN_BUFFER_LEN)? };
                Ok((value as $ty, consumed))
            }

            /// Tries to decode from the currently available bytes without bounds checks.
            ///
            /// This internal entry point is used by buffered readers that may have
            /// only part of a variable-length payload in their input buffer. It
            /// decodes while scanning for the terminating byte, so callers do not
            /// need a separate terminator pre-scan before calling the codec.
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
            /// `Err((error, consumed))` when the payload is invalid and should be
            /// consumed before the error is reported.
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
            ) -> Result<Option<($ty, usize)>, (Leb128DecodeError, usize)> {
                // SAFETY: The caller guarantees that exactly `available` bytes
                // are readable from `index`.
                unsafe {
                    read_uleb_available_unchecked::<P>(
                        input,
                        index,
                        <$ty>::BITS,
                        Self::REQUIRED_MIN_BUFFER_LEN,
                        available,
                    )
                }
                .map(|result| result.map(|(value, consumed)| (value as $ty, consumed)))
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
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) -> usize {
                // SAFETY: The caller guarantees enough writable bytes for this type.
                unsafe { write_uleb_unchecked(output, index, value as u128) }
            }
        }
    };
}

macro_rules! impl_signed_leb128_codec {
    ($ty:ty) => {
        impl<P> Leb128Codec<$ty, P>
        where
            P: DecodePolicy,
        {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = (<$ty>::BITS as usize).div_ceil(7);

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
            /// Returns [`Leb128DecodeError`] if the bytes are malformed or strict
            /// decoding rejects a non-canonical representation.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that `input.as_ptr().add(index)` is valid to
            /// read [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes, or that a valid terminating byte
            /// appears before that limit.
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> Result<($ty, usize), Leb128DecodeError> {
                // SAFETY: The caller guarantees enough readable bytes for this type.
                let (value, consumed) =
                    unsafe { read_sleb_unchecked::<P>(input, index, <$ty>::BITS, Self::REQUIRED_MIN_BUFFER_LEN)? };
                Ok((value as $ty, consumed))
            }

            /// Tries to decode from the currently available bytes without bounds checks.
            ///
            /// This internal entry point is used by buffered readers that may have
            /// only part of a variable-length payload in their input buffer. It
            /// decodes while scanning for the terminating byte, so callers do not
            /// need a separate terminator pre-scan before calling the codec.
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
            /// `Err((error, consumed))` when the payload is invalid and should be
            /// consumed before the error is reported.
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
            ) -> Result<Option<($ty, usize)>, (Leb128DecodeError, usize)> {
                // SAFETY: The caller guarantees that exactly `available` bytes
                // are readable from `index`.
                unsafe {
                    read_sleb_available_unchecked::<P>(
                        input,
                        index,
                        <$ty>::BITS,
                        Self::REQUIRED_MIN_BUFFER_LEN,
                        available,
                    )
                }
                .map(|result| result.map(|(value, consumed)| (value as $ty, consumed)))
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
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) -> usize {
                // SAFETY: The caller guarantees enough writable bytes for this type.
                unsafe { write_sleb_unchecked(output, index, value as i128) }
            }
        }
    };
}

impl_unsigned_leb128_codec!(u8);
impl_unsigned_leb128_codec!(u16);
impl_unsigned_leb128_codec!(u32);
impl_unsigned_leb128_codec!(u64);
impl_unsigned_leb128_codec!(u128);
impl_unsigned_leb128_codec!(usize);

impl_signed_leb128_codec!(i8);
impl_signed_leb128_codec!(i16);
impl_signed_leb128_codec!(i32);
impl_signed_leb128_codec!(i64);
impl_signed_leb128_codec!(i128);
impl_signed_leb128_codec!(isize);

#[inline(always)]
unsafe fn read_uleb_unchecked<P>(
    input: &[u8],
    index: usize,
    bits: u32,
    max_bytes: usize,
) -> Result<(u128, usize), Leb128DecodeError>
where
    P: DecodePolicy,
{
    // SAFETY: The caller guarantees enough readable bytes for the full maximum
    // payload width, which is exactly the `available` value passed here.
    match unsafe { read_uleb_available_unchecked::<P>(input, index, bits, max_bytes, max_bytes) } {
        Ok(Some(value)) => Ok(value),
        Ok(None) => unreachable!("maximum-width LEB128 input must complete or fail"),
        Err((error, _consumed)) => Err(error),
    }
}

#[inline(always)]
unsafe fn read_uleb_available_unchecked<P>(
    input: &[u8],
    index: usize,
    bits: u32,
    max_bytes: usize,
    available: usize,
) -> Result<Option<(u128, usize)>, (Leb128DecodeError, usize)>
where
    P: DecodePolicy,
{
    debug_assert!(available <= max_bytes, "available bytes exceed LEB128 maximum width");
    let mut value = 0u128;
    let mut shift = 0u32;
    for offset in 0..available {
        // SAFETY: The caller guarantees enough readable bytes for this loop.
        let byte = unsafe { *input.as_ptr().add(index + offset) };
        let payload = u128::from(byte & 0x7F);
        value |= payload << shift;
        if byte & 0x80 == 0 {
            if offset == max_bytes - 1 && !unsigned_final_payload_fits(byte, bits, offset) {
                let kind = Leb128DecodeErrorKind::Malformed;
                return Err((Leb128DecodeError::new(kind, index + offset), offset + 1));
            }
            let consumed = offset + 1;
            if P::STRICT && !has_canonical_uleb_len(value, consumed) {
                let kind = Leb128DecodeErrorKind::NonCanonical;
                return Err((Leb128DecodeError::new(kind, index), consumed));
            }
            return Ok(Some((value, consumed)));
        }
        shift += 7;
    }
    if available < max_bytes {
        return Ok(None);
    }
    let kind = Leb128DecodeErrorKind::Malformed;
    Err((Leb128DecodeError::new(kind, index + max_bytes - 1), max_bytes))
}

#[inline(always)]
unsafe fn read_sleb_unchecked<P>(
    input: &[u8],
    index: usize,
    bits: u32,
    max_bytes: usize,
) -> Result<(i128, usize), Leb128DecodeError>
where
    P: DecodePolicy,
{
    // SAFETY: The caller guarantees enough readable bytes for the full maximum
    // payload width, which is exactly the `available` value passed here.
    match unsafe { read_sleb_available_unchecked::<P>(input, index, bits, max_bytes, max_bytes) } {
        Ok(Some(value)) => Ok(value),
        Ok(None) => unreachable!("maximum-width LEB128 input must complete or fail"),
        Err((error, _consumed)) => Err(error),
    }
}

#[inline(always)]
unsafe fn read_sleb_available_unchecked<P>(
    input: &[u8],
    index: usize,
    bits: u32,
    max_bytes: usize,
    available: usize,
) -> Result<Option<(i128, usize)>, (Leb128DecodeError, usize)>
where
    P: DecodePolicy,
{
    debug_assert!(available <= max_bytes, "available bytes exceed LEB128 maximum width");
    let mut value = 0i128;
    let mut shift = 0u32;
    for offset in 0..available {
        // SAFETY: The caller guarantees enough readable bytes for this loop.
        let byte = unsafe { *input.as_ptr().add(index + offset) };
        let payload = i128::from(byte & 0x7F);
        value |= payload << shift;
        if byte & 0x80 == 0 {
            if offset == max_bytes - 1 && !signed_final_payload_fits(byte, bits, offset) {
                let kind = Leb128DecodeErrorKind::Malformed;
                return Err((Leb128DecodeError::new(kind, index + offset), offset + 1));
            }
            if byte & 0x40 != 0 && shift + 7 < i128::BITS {
                value |= (!0i128) << (shift + 7);
            }
            let consumed = offset + 1;
            if P::STRICT && !has_canonical_sleb_len(value, consumed) {
                let kind = Leb128DecodeErrorKind::NonCanonical;
                return Err((Leb128DecodeError::new(kind, index), consumed));
            }
            return Ok(Some((value, consumed)));
        }
        shift += 7;
    }
    if available < max_bytes {
        return Ok(None);
    }
    let kind = Leb128DecodeErrorKind::Malformed;
    Err((Leb128DecodeError::new(kind, index + max_bytes - 1), max_bytes))
}

#[must_use]
#[inline(always)]
fn unsigned_final_payload_fits(byte: u8, bits: u32, offset: usize) -> bool {
    let used_bits = bits - offset as u32 * 7;
    byte >> used_bits == 0
}

#[must_use]
#[inline(always)]
fn signed_final_payload_fits(byte: u8, bits: u32, offset: usize) -> bool {
    let used_bits = bits - offset as u32 * 7;
    let payload = byte & 0x7F;
    let used_mask = ((1u16 << used_bits) - 1) as u8;
    let unused_mask = 0x7F & !used_mask;
    let sign_bit = 1u8 << (used_bits - 1);
    if payload & sign_bit == 0 {
        payload & unused_mask == 0
    } else {
        payload & unused_mask == unused_mask
    }
}

/// Checks whether an unsigned LEB128 value used its canonical encoded length.
///
/// # Parameters
///
/// - `value`: Decoded unsigned value.
/// - `actual_len`: Number of bytes consumed from the input.
///
/// # Returns
///
/// Returns `true` if `actual_len` is the canonical encoded length of `value`.
#[must_use]
#[inline(always)]
fn has_canonical_uleb_len(value: u128, actual_len: usize) -> bool {
    canonical_uleb_len(value) == actual_len
}

/// Checks whether a signed LEB128 value used its canonical encoded length.
///
/// # Parameters
///
/// - `value`: Decoded signed value.
/// - `actual_len`: Number of bytes consumed from the input.
///
/// # Returns
///
/// Returns `true` if `actual_len` is the canonical encoded length of `value`.
#[must_use]
#[inline(always)]
fn has_canonical_sleb_len(value: i128, actual_len: usize) -> bool {
    canonical_sleb_len(value) == actual_len
}

/// Computes the canonical unsigned LEB128 encoded length.
///
/// # Parameters
///
/// - `value`: Unsigned value to measure.
///
/// # Returns
///
/// Returns the number of bytes used by the canonical unsigned LEB128 encoding.
#[must_use]
#[inline(always)]
fn canonical_uleb_len(mut value: u128) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

/// Computes the canonical signed LEB128 encoded length.
///
/// # Parameters
///
/// - `value`: Signed value to measure.
///
/// # Returns
///
/// Returns the number of bytes used by the canonical signed LEB128 encoding.
#[must_use]
#[inline(always)]
fn canonical_sleb_len(mut value: i128) -> usize {
    let mut len = 0;
    loop {
        let byte = (value & 0x7F) as u8;
        let sign_bit_set = byte & 0x40 != 0;
        value >>= 7;
        len += 1;
        if (value == 0 && !sign_bit_set) || (value == -1 && sign_bit_set) {
            return len;
        }
    }
}

unsafe fn write_uleb_unchecked(output: &mut [u8], index: usize, mut value: u128) -> usize {
    let mut offset = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        // SAFETY: The caller guarantees enough writable bytes for the encoded value.
        unsafe {
            *output.as_mut_ptr().add(index + offset) = byte;
        }
        offset += 1;
        if value == 0 {
            return offset;
        }
    }
}

unsafe fn write_sleb_unchecked(output: &mut [u8], index: usize, mut value: i128) -> usize {
    let mut offset = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        let sign_bit_set = byte & 0x40 != 0;
        value >>= 7;
        let done = (value == 0 && !sign_bit_set) || (value == -1 && sign_bit_set);
        if !done {
            byte |= 0x80;
        }
        // SAFETY: The caller guarantees enough writable bytes for the encoded value.
        unsafe {
            *output.as_mut_ptr().add(index + offset) = byte;
        }
        offset += 1;
        if done {
            return offset;
        }
    }
}
