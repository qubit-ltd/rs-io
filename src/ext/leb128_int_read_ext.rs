/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
};

use crate::BinaryReadExt;

/// Extension methods for reading LEB128 encoded integers.
///
/// Unsigned methods read unsigned LEB128 values, and signed methods read signed
/// LEB128 values. Both forms encode seven payload bits per byte in
/// least-significant group first order, with the high bit marking
/// continuation. The integer encoding is described by the WebAssembly Core
/// binary format:
/// <https://webassembly.github.io/spec/core/binary/values.html#integers>.
///
/// Methods with the `_strict` suffix also reject non-canonical encodings, such
/// as values encoded with unnecessary continuation bytes.
pub trait Leb128IntReadExt: Read {
    /// Reads an unsigned LEB128 `u8`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u8(&mut self) -> Result<u8>;

    /// Reads a canonical unsigned LEB128 `u8`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_u8_strict(&mut self) -> Result<u8>;

    /// Reads an unsigned LEB128 `u16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u16(&mut self) -> Result<u16>;

    /// Reads a canonical unsigned LEB128 `u16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_u16_strict(&mut self) -> Result<u16>;

    /// Reads an unsigned LEB128 `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u32(&mut self) -> Result<u32>;

    /// Reads a canonical unsigned LEB128 `u32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_u32_strict(&mut self) -> Result<u32>;

    /// Reads an unsigned LEB128 `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u64(&mut self) -> Result<u64>;

    /// Reads a canonical unsigned LEB128 `u64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_u64_strict(&mut self) -> Result<u64>;

    /// Reads an unsigned LEB128 `u128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_u128(&mut self) -> Result<u128>;

    /// Reads a canonical unsigned LEB128 `u128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_u128_strict(&mut self) -> Result<u128>;

    /// Reads an unsigned LEB128 `usize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_uleb_usize(&mut self) -> Result<usize>;

    /// Reads a canonical unsigned LEB128 `usize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_uleb_usize_strict(&mut self) -> Result<usize>;

    /// Reads a signed LEB128 `i8`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i8(&mut self) -> Result<i8>;

    /// Reads a canonical signed LEB128 `i8`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_i8_strict(&mut self) -> Result<i8>;

    /// Reads a signed LEB128 `i16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i16(&mut self) -> Result<i16>;

    /// Reads a canonical signed LEB128 `i16`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_i16_strict(&mut self) -> Result<i16>;

    /// Reads a signed LEB128 `i32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i32(&mut self) -> Result<i32>;

    /// Reads a canonical signed LEB128 `i32`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_i32_strict(&mut self) -> Result<i32>;

    /// Reads a signed LEB128 `i64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i64(&mut self) -> Result<i64>;

    /// Reads a canonical signed LEB128 `i64`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_i64_strict(&mut self) -> Result<i64>;

    /// Reads a signed LEB128 `i128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_i128(&mut self) -> Result<i128>;

    /// Reads a canonical signed LEB128 `i128`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_i128_strict(&mut self) -> Result<i128>;

    /// Reads a signed LEB128 `isize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow,
    /// or another I/O error from the underlying reader.
    fn read_sleb_isize(&mut self) -> Result<isize>;

    /// Reads a canonical signed LEB128 `isize`.
    ///
    /// # Returns
    /// The decoded value.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` for truncated input, `InvalidData` for overflow
    /// or non-canonical encoding, or another I/O error from the underlying reader.
    fn read_sleb_isize_strict(&mut self) -> Result<isize>;
}

impl<T> Leb128IntReadExt for T
where
    T: Read,
{
    #[inline]
    fn read_uleb_u8(&mut self) -> Result<u8> {
        read_uleb(self, u8::BITS, "u8", false).map(u128_to_u8)
    }

    #[inline]
    fn read_uleb_u8_strict(&mut self) -> Result<u8> {
        read_uleb(self, u8::BITS, "u8", true).map(u128_to_u8)
    }

    #[inline]
    fn read_uleb_u16(&mut self) -> Result<u16> {
        read_uleb(self, u16::BITS, "u16", false).map(u128_to_u16)
    }

    #[inline]
    fn read_uleb_u16_strict(&mut self) -> Result<u16> {
        read_uleb(self, u16::BITS, "u16", true).map(u128_to_u16)
    }

    #[inline]
    fn read_uleb_u32(&mut self) -> Result<u32> {
        read_uleb(self, u32::BITS, "u32", false).map(u128_to_u32)
    }

    #[inline]
    fn read_uleb_u32_strict(&mut self) -> Result<u32> {
        read_uleb(self, u32::BITS, "u32", true).map(u128_to_u32)
    }

    #[inline]
    fn read_uleb_u64(&mut self) -> Result<u64> {
        read_uleb(self, u64::BITS, "u64", false).map(u128_to_u64)
    }

    #[inline]
    fn read_uleb_u64_strict(&mut self) -> Result<u64> {
        read_uleb(self, u64::BITS, "u64", true).map(u128_to_u64)
    }

    #[inline]
    fn read_uleb_u128(&mut self) -> Result<u128> {
        read_uleb(self, u128::BITS, "u128", false)
    }

    #[inline]
    fn read_uleb_u128_strict(&mut self) -> Result<u128> {
        read_uleb(self, u128::BITS, "u128", true)
    }

    #[inline]
    fn read_uleb_usize(&mut self) -> Result<usize> {
        read_uleb(self, usize::BITS, "usize", false).map(u128_to_usize)
    }

    #[inline]
    fn read_uleb_usize_strict(&mut self) -> Result<usize> {
        read_uleb(self, usize::BITS, "usize", true).map(u128_to_usize)
    }

    #[inline]
    fn read_sleb_i8(&mut self) -> Result<i8> {
        read_sleb(self, i8::BITS, "i8", false).map(i128_to_i8)
    }

    #[inline]
    fn read_sleb_i8_strict(&mut self) -> Result<i8> {
        read_sleb(self, i8::BITS, "i8", true).map(i128_to_i8)
    }

    #[inline]
    fn read_sleb_i16(&mut self) -> Result<i16> {
        read_sleb(self, i16::BITS, "i16", false).map(i128_to_i16)
    }

    #[inline]
    fn read_sleb_i16_strict(&mut self) -> Result<i16> {
        read_sleb(self, i16::BITS, "i16", true).map(i128_to_i16)
    }

    #[inline]
    fn read_sleb_i32(&mut self) -> Result<i32> {
        read_sleb(self, i32::BITS, "i32", false).map(i128_to_i32)
    }

    #[inline]
    fn read_sleb_i32_strict(&mut self) -> Result<i32> {
        read_sleb(self, i32::BITS, "i32", true).map(i128_to_i32)
    }

    #[inline]
    fn read_sleb_i64(&mut self) -> Result<i64> {
        read_sleb(self, i64::BITS, "i64", false).map(i128_to_i64)
    }

    #[inline]
    fn read_sleb_i64_strict(&mut self) -> Result<i64> {
        read_sleb(self, i64::BITS, "i64", true).map(i128_to_i64)
    }

    #[inline]
    fn read_sleb_i128(&mut self) -> Result<i128> {
        read_sleb(self, i128::BITS, "i128", false)
    }

    #[inline]
    fn read_sleb_i128_strict(&mut self) -> Result<i128> {
        read_sleb(self, i128::BITS, "i128", true)
    }

    #[inline]
    fn read_sleb_isize(&mut self) -> Result<isize> {
        read_sleb(self, isize::BITS, "isize", false).map(i128_to_isize)
    }

    #[inline]
    fn read_sleb_isize_strict(&mut self) -> Result<isize> {
        read_sleb(self, isize::BITS, "isize", true).map(i128_to_isize)
    }
}

#[inline]
fn u128_to_u8(value: u128) -> u8 {
    value as u8
}

#[inline]
fn u128_to_u16(value: u128) -> u16 {
    value as u16
}

#[inline]
fn u128_to_u32(value: u128) -> u32 {
    value as u32
}

#[inline]
fn u128_to_u64(value: u128) -> u64 {
    value as u64
}

#[inline]
fn u128_to_usize(value: u128) -> usize {
    value as usize
}

#[inline]
fn i128_to_i8(value: i128) -> i8 {
    value as i8
}

#[inline]
fn i128_to_i16(value: i128) -> i16 {
    value as i16
}

#[inline]
fn i128_to_i32(value: i128) -> i32 {
    value as i32
}

#[inline]
fn i128_to_i64(value: i128) -> i64 {
    value as i64
}

#[inline]
fn i128_to_isize(value: i128) -> isize {
    value as isize
}

/// Decoded unsigned LEB128 value and its raw bytes.
struct DecodedUleb {
    value: u128,
    bytes: Vec<u8>,
}

/// Decoded signed LEB128 value and its raw bytes.
struct DecodedSleb {
    value: i128,
    bytes: Vec<u8>,
}

/// Reads an unsigned LEB128 integer constrained to `bits`.
///
/// # Parameters
/// - `reader`: Source reader. It may be a sized reader or a reader trait
///   object.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
/// - `strict`: Whether to reject non-canonical encodings.
///
/// # Returns
/// Decoded value as `u128`.
///
/// # Errors
/// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed,
/// overflowing, or non-canonical input, or another I/O error from `reader`.
fn read_uleb(
    reader: &mut dyn Read,
    bits: u32,
    type_name: &'static str,
    strict: bool,
) -> Result<u128> {
    let decoded = read_uleb_with_bytes(reader, bits, type_name)?;
    if strict && !is_canonical_uleb(decoded.value, &decoded.bytes) {
        return Err(noncanonical_leb128(type_name));
    }
    Ok(decoded.value)
}

/// Reads an unsigned LEB128 integer and keeps its raw bytes.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
///
/// # Returns
/// Decoded unsigned value and raw bytes.
///
/// # Errors
/// Returns an I/O error, truncated input error, or malformed data error.
fn read_uleb_with_bytes(
    reader: &mut dyn Read,
    bits: u32,
    type_name: &'static str,
) -> Result<DecodedUleb> {
    let max_bytes = bits.div_ceil(7);
    let final_payload_bits = bits - (max_bytes - 1) * 7;
    let max_last_payload = ((1u16 << final_payload_bits) - 1) as u8;

    let mut value = 0u128;
    let mut bytes = Vec::with_capacity(max_bytes as usize);
    for index in 0..max_bytes {
        let byte = reader.read_u8()?;
        let payload = byte & 0x7f;
        let is_too_wide_final_byte = (index == max_bytes - 1) && (payload > max_last_payload);
        if is_too_wide_final_byte {
            return Err(invalid_leb128(type_name));
        }
        value |= (payload as u128) << (index * 7);
        bytes.push(byte);
        if byte & 0x80 == 0 {
            return Ok(DecodedUleb { value, bytes });
        }
    }
    Err(invalid_leb128(type_name))
}

/// Reads a signed LEB128 integer constrained to `bits`.
///
/// # Parameters
/// - `reader`: Source reader. It may be a sized reader or a reader trait
///   object.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
/// - `strict`: Whether to reject non-canonical encodings.
///
/// # Returns
/// Decoded value as `i128`.
///
/// # Errors
/// Returns `UnexpectedEof` for truncated input, `InvalidData` for malformed,
/// overflowing, or non-canonical input, or another I/O error from `reader`.
fn read_sleb(
    reader: &mut dyn Read,
    bits: u32,
    type_name: &'static str,
    strict: bool,
) -> Result<i128> {
    let decoded = read_sleb_with_bytes(reader, bits, type_name)?;
    if strict && !is_canonical_sleb(decoded.value, &decoded.bytes) {
        return Err(noncanonical_leb128(type_name));
    }
    Ok(decoded.value)
}

/// Reads a signed LEB128 integer and keeps its raw bytes.
///
/// # Parameters
/// - `reader`: Source reader.
/// - `bits`: Target integer width in bits.
/// - `type_name`: Target type name used in error messages.
///
/// # Returns
/// Decoded signed value and raw bytes.
///
/// # Errors
/// Returns an I/O error, truncated input error, or malformed data error.
fn read_sleb_with_bytes(
    reader: &mut dyn Read,
    bits: u32,
    type_name: &'static str,
) -> Result<DecodedSleb> {
    let max_bytes = bits.div_ceil(7);
    let mut value = 0i128;
    let mut shift = 0u32;
    let mut bytes = Vec::with_capacity(max_bytes as usize);
    for index in 0..max_bytes {
        let byte = reader.read_u8()?;
        let payload = byte & 0x7f;
        if is_too_wide_signed_final_payload(payload, index, bits) {
            return Err(invalid_leb128(type_name));
        }

        value |= (payload as i128) << shift;
        shift += 7;
        bytes.push(byte);
        if byte & 0x80 == 0 {
            if shift < i128::BITS && byte & 0x40 != 0 {
                value |= (!0i128) << shift;
            }
            return Ok(DecodedSleb { value, bytes });
        }
    }
    Err(invalid_leb128(type_name))
}

/// Returns whether the final signed payload byte exceeds the target width.
///
/// # Parameters
/// - `payload`: Seven-bit payload from the current byte.
/// - `index`: Zero-based byte index.
/// - `bits`: Target signed integer width in bits.
///
/// # Returns
/// `true` when this final payload cannot represent a valid value of the target
/// width.
fn is_too_wide_signed_final_payload(payload: u8, index: u32, bits: u32) -> bool {
    let max_bytes = bits.div_ceil(7);
    if index != max_bytes - 1 {
        return false;
    }

    let used_bits = bits - index * 7;
    let sign_mask = 1u8 << (used_bits - 1);
    let used_mask = (1u8 << used_bits) - 1;
    let unused_mask = 0x7f_u8 & !used_mask;
    let unused_bits = payload & unused_mask;
    if payload & sign_mask == 0 {
        unused_bits != 0
    } else {
        unused_bits != unused_mask
    }
}

/// Checks whether `bytes` are the canonical unsigned LEB128 encoding.
///
/// # Parameters
/// - `value`: Decoded value.
/// - `bytes`: Raw bytes read from the stream.
///
/// # Returns
/// `true` when re-encoding `value` produces the same bytes.
fn is_canonical_uleb(value: u128, bytes: &[u8]) -> bool {
    let mut expected = Vec::new();
    encode_uleb(value, &mut expected);
    expected == bytes
}

/// Checks whether `bytes` are the canonical signed LEB128 encoding.
///
/// # Parameters
/// - `value`: Decoded value.
/// - `bytes`: Raw bytes read from the stream.
///
/// # Returns
/// `true` when re-encoding `value` produces the same bytes.
fn is_canonical_sleb(value: i128, bytes: &[u8]) -> bool {
    let mut expected = Vec::new();
    encode_sleb(value, &mut expected);
    expected == bytes
}

/// Encodes an unsigned LEB128 value into `output`.
///
/// # Parameters
/// - `value`: Value to encode.
/// - `output`: Destination buffer.
fn encode_uleb(mut value: u128, output: &mut Vec<u8>) {
    while value > 0x7f {
        output.push(((value as u8) & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

/// Encodes a signed LEB128 value into `output`.
///
/// # Parameters
/// - `value`: Value to encode.
/// - `output`: Destination buffer.
fn encode_sleb(value: i128, output: &mut Vec<u8>) {
    let mut remaining = value;
    loop {
        let byte = (remaining as u8) & 0x7f;
        remaining >>= 7;
        let is_done = (remaining == 0 && byte & 0x40 == 0) || (remaining == -1 && byte & 0x40 != 0);
        if is_done {
            output.push(byte);
            return;
        }
        output.push(byte | 0x80);
    }
}

/// Builds an invalid-data error for malformed LEB128 integers.
///
/// # Parameters
/// - `type_name`: Target type name.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error.
#[inline]
fn invalid_leb128(type_name: &'static str) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("malformed LEB128 integer for {type_name}"),
    )
}

/// Builds an invalid-data error for non-canonical LEB128 integers.
///
/// # Parameters
/// - `type_name`: Target type name.
///
/// # Returns
/// An [`ErrorKind::InvalidData`] error.
#[inline]
fn noncanonical_leb128(type_name: &'static str) -> Error {
    Error::new(
        ErrorKind::InvalidData,
        format!("non-canonical LEB128 integer for {type_name}"),
    )
}
