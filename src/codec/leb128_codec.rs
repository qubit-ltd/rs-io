/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::{
    Leb128DecodeError,
    Leb128DecodeErrorKind,
};

const MAX_LEB128_BYTES: usize = 19;
const MAX_U16_LEB128_BYTES: usize = 3;
const MAX_U32_LEB128_BYTES: usize = 5;
const MAX_U64_LEB128_BYTES: usize = 10;
const MAX_U128_LEB128_BYTES: usize = 19;

/// Buffer-level codec for unsigned and signed LEB128 integers.
///
/// The codec does not perform `std::io::Read` or `std::io::Write`; it works
/// directly on caller-owned byte slices and reports how many bytes were consumed
/// or written. Unsigned integer methods use unsigned LEB128. Signed integer
/// methods use signed LEB128.
///
/// Checked `*_at` methods validate the supplied slice range and return
/// `Ok(None)` when the input ends before a complete value is available. The
/// unchecked methods skip slice bounds checks and are intended for validated hot
/// paths.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Leb128Codec {
    strict: bool,
}

impl Leb128Codec {
    /// Creates a non-strict LEB128 codec.
    ///
    /// # Returns
    ///
    /// Returns a codec that accepts non-canonical encodings when decoding.
    #[inline]
    pub const fn new() -> Self {
        Self { strict: false }
    }

    /// Creates a LEB128 codec with an explicit canonical decoding policy.
    ///
    /// # Parameters
    ///
    /// - `strict`: Whether to reject non-canonical encodings.
    ///
    /// # Returns
    ///
    /// Returns a codec configured with the supplied policy.
    #[inline]
    pub const fn with_strict(strict: bool) -> Self {
        Self { strict }
    }

    /// Reports whether strict canonical decoding is enabled.
    ///
    /// # Returns
    ///
    /// Returns `true` when decode methods reject non-canonical encodings, and
    /// `false` when they accept any well-formed representation.
    #[must_use]
    #[inline]
    pub const fn strict(self) -> bool {
        self.strict
    }

    /// Updates the canonical decoding policy.
    ///
    /// # Parameters
    ///
    /// - `strict`: Whether to reject non-canonical encodings.
    #[inline]
    pub fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
    }

    /// Reads a `u16` value from a three-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width unsigned LEB128 buffer for a `u16` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_u16_from_array(self, input: [u8; 3]) -> Result<(u16, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width u16 LEB128 range.
        unsafe { self.read_u16_at_unchecked(&input, 0) }
    }

    /// Reads a `u32` value from a five-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width unsigned LEB128 buffer for a `u32` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_u32_from_array(self, input: [u8; 5]) -> Result<(u32, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width u32 LEB128 range.
        unsafe { self.read_u32_at_unchecked(&input, 0) }
    }

    /// Reads a `u64` value from a ten-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width unsigned LEB128 buffer for a `u64` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_u64_from_array(self, input: [u8; 10]) -> Result<(u64, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width u64 LEB128 range.
        unsafe { self.read_u64_at_unchecked(&input, 0) }
    }

    /// Reads a `u128` value from a nineteen-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width unsigned LEB128 buffer for a `u128` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_u128_from_array(self, input: [u8; 19]) -> Result<(u128, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width u128 LEB128 range.
        unsafe { self.read_u128_at_unchecked(&input, 0) }
    }

    /// Reads an `i16` value from a three-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width signed LEB128 buffer for an `i16` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_i16_from_array(self, input: [u8; 3]) -> Result<(i16, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width i16 LEB128 range.
        unsafe { self.read_i16_at_unchecked(&input, 0) }
    }

    /// Reads an `i32` value from a five-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width signed LEB128 buffer for an `i32` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_i32_from_array(self, input: [u8; 5]) -> Result<(i32, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width i32 LEB128 range.
        unsafe { self.read_i32_at_unchecked(&input, 0) }
    }

    /// Reads an `i64` value from a ten-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width signed LEB128 buffer for an `i64` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_i64_from_array(self, input: [u8; 10]) -> Result<(i64, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width i64 LEB128 range.
        unsafe { self.read_i64_at_unchecked(&input, 0) }
    }

    /// Reads an `i128` value from a nineteen-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `input`: A maximum-width signed LEB128 buffer for an `i128` value.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] if `input` contains a malformed value or,
    /// when strict mode is enabled, a non-canonical value.
    #[inline]
    pub fn read_i128_from_array(self, input: [u8; 19]) -> Result<(i128, usize), Leb128DecodeError> {
        // SAFETY: The array has the full maximum-width i128 LEB128 range.
        unsafe { self.read_i128_at_unchecked(&input, 0) }
    }

    /// Reads a `u16` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `u16` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_u16_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(u16, usize)>, Leb128DecodeError> {
        self.read_uleb_at(input, index, u16::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as u16, consumed)))
    }

    /// Reads a `u32` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `u32` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_u32_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(u32, usize)>, Leb128DecodeError> {
        self.read_uleb_at(input, index, u32::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as u32, consumed)))
    }

    /// Reads a `u64` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `u64` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_u64_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(u64, usize)>, Leb128DecodeError> {
        self.read_uleb_at(input, index, u64::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as u64, consumed)))
    }

    /// Reads a `u128` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `u128` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_u128_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(u128, usize)>, Leb128DecodeError> {
        self.read_uleb_at(input, index, u128::BITS)
    }

    /// Reads an `i16` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `i16` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_i16_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(i16, usize)>, Leb128DecodeError> {
        self.read_sleb_at(input, index, i16::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as i16, consumed)))
    }

    /// Reads an `i32` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `i32` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_i32_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(i32, usize)>, Leb128DecodeError> {
        self.read_sleb_at(input, index, i32::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as i32, consumed)))
    }

    /// Reads an `i64` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `i64` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_i64_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(i64, usize)>, Leb128DecodeError> {
        self.read_sleb_at(input, index, i64::BITS)
            .map(|decoded| decoded.map(|(value, consumed)| (value as i64, consumed)))
    }

    /// Reads an `i128` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some((value, consumed)))` for a complete value, `Ok(None)`
    /// when the slice ends before the value is complete, and `Err` for malformed
    /// or non-canonical input.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when `index` is out of range, the encoded
    /// value exceeds the `i128` width, or strict mode rejects a non-canonical
    /// encoding.
    #[inline]
    pub fn read_i128_at(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<Option<(i128, usize)>, Leb128DecodeError> {
        self.read_sleb_at(input, index, i128::BITS)
    }

    /// Reads a `u16` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 3` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_u16_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(u16, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full u16 range is in bounds.
        unsafe { self.read_uleb_at_unchecked(input, index, u16::BITS) }
            .map(|(value, consumed)| (value as u16, consumed))
    }

    /// Reads a `u32` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 5` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_u32_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(u32, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full u32 range is in bounds.
        unsafe { self.read_uleb_at_unchecked(input, index, u32::BITS) }
            .map(|(value, consumed)| (value as u32, consumed))
    }

    /// Reads a `u64` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 10` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_u64_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(u64, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full u64 range is in bounds.
        unsafe { self.read_uleb_at_unchecked(input, index, u64::BITS) }
            .map(|(value, consumed)| (value as u64, consumed))
    }

    /// Reads a `u128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 19` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_u128_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(u128, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full u128 range is in bounds.
        unsafe { self.read_uleb_at_unchecked(input, index, u128::BITS) }
    }

    /// Reads an `i16` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 3` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_i16_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(i16, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full i16 range is in bounds.
        unsafe { self.read_sleb_at_unchecked(input, index, i16::BITS) }
            .map(|(value, consumed)| (value as i16, consumed))
    }

    /// Reads an `i32` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 5` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_i32_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(i32, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full i32 range is in bounds.
        unsafe { self.read_sleb_at_unchecked(input, index, i32::BITS) }
            .map(|(value, consumed)| (value as i32, consumed))
    }

    /// Reads an `i64` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 10` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_i64_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(i64, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full i64 range is in bounds.
        unsafe { self.read_sleb_at_unchecked(input, index, i64::BITS) }
            .map(|(value, consumed)| (value as i64, consumed))
    }

    /// Reads an `i128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded value and the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Leb128DecodeError`] when the validated range contains malformed
    /// data or, in strict mode, a non-canonical encoding.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 19` is in bounds for
    /// `input`.
    #[inline]
    pub unsafe fn read_i128_at_unchecked(
        self,
        input: &[u8],
        index: usize,
    ) -> Result<(i128, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full i128 range is in bounds.
        unsafe { self.read_sleb_at_unchecked(input, index, i128::BITS) }
    }

    /// Encodes a `u16` value into a three-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn u16_bytes(self, value: u16) -> ([u8; 3], usize) {
        let mut output = [0_u8; MAX_U16_LEB128_BYTES];
        let count = encode_uleb(value as u128, &mut output);
        (output, count)
    }

    /// Encodes a `u32` value into a five-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn u32_bytes(self, value: u32) -> ([u8; 5], usize) {
        let mut output = [0_u8; MAX_U32_LEB128_BYTES];
        let count = encode_uleb(value as u128, &mut output);
        (output, count)
    }

    /// Encodes a `u64` value into a ten-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn u64_bytes(self, value: u64) -> ([u8; 10], usize) {
        let mut output = [0_u8; MAX_U64_LEB128_BYTES];
        let count = encode_uleb(value as u128, &mut output);
        (output, count)
    }

    /// Encodes a `u128` value into a nineteen-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn u128_bytes(self, value: u128) -> ([u8; 19], usize) {
        let mut output = [0_u8; MAX_U128_LEB128_BYTES];
        let count = encode_uleb(value, &mut output);
        (output, count)
    }

    /// Encodes an `i16` value into a three-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn i16_bytes(self, value: i16) -> ([u8; 3], usize) {
        let mut output = [0_u8; MAX_U16_LEB128_BYTES];
        let count = encode_sleb(value as i128, &mut output);
        (output, count)
    }

    /// Encodes an `i32` value into a five-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn i32_bytes(self, value: i32) -> ([u8; 5], usize) {
        let mut output = [0_u8; MAX_U32_LEB128_BYTES];
        let count = encode_sleb(value as i128, &mut output);
        (output, count)
    }

    /// Encodes an `i64` value into a ten-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn i64_bytes(self, value: i64) -> ([u8; 10], usize) {
        let mut output = [0_u8; MAX_U64_LEB128_BYTES];
        let count = encode_sleb(value as i128, &mut output);
        (output, count)
    }

    /// Encodes an `i128` value into a nineteen-byte maximum-width array.
    ///
    /// # Parameters
    ///
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the array and the number of initialized bytes.
    #[must_use]
    #[inline]
    pub fn i128_bytes(self, value: i128) -> ([u8; 19], usize) {
        let mut output = [0_u8; MAX_U128_LEB128_BYTES];
        let count = encode_sleb(value, &mut output);
        (output, count)
    }

    /// Writes a `u16` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_u16_at(self, output: &mut [u8], index: usize, value: u16) -> Option<usize> {
        let (encoded, count) = self.u16_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes a `u32` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_u32_at(self, output: &mut [u8], index: usize, value: u32) -> Option<usize> {
        let (encoded, count) = self.u32_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes a `u64` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_u64_at(self, output: &mut [u8], index: usize, value: u64) -> Option<usize> {
        let (encoded, count) = self.u64_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes a `u128` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_u128_at(self, output: &mut [u8], index: usize, value: u128) -> Option<usize> {
        let (encoded, count) = self.u128_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes an `i16` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_i16_at(self, output: &mut [u8], index: usize, value: i16) -> Option<usize> {
        let (encoded, count) = self.i16_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes an `i32` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_i32_at(self, output: &mut [u8], index: usize, value: i32) -> Option<usize> {
        let (encoded, count) = self.i32_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes an `i64` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_i64_at(self, output: &mut [u8], index: usize, value: i64) -> Option<usize> {
        let (encoded, count) = self.i64_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes an `i128` value at `index`.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns `Some(consumed)` with the number of bytes written when the
    /// destination has enough capacity. Returns `None` when `index` is out of
    /// bounds or the encoded value would not fit.
    #[inline]
    pub fn write_i128_at(self, output: &mut [u8], index: usize, value: i128) -> Option<usize> {
        let (encoded, count) = self.i128_bytes(value);
        write_encoded_at(output, index, &encoded[..count])
    }

    /// Writes a `u16` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 3` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_u16_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: u16,
    ) -> usize {
        let (encoded, count) = self.u16_bytes(value);
        // SAFETY: The caller guarantees the full u16 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes a `u32` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 5` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_u32_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: u32,
    ) -> usize {
        let (encoded, count) = self.u32_bytes(value);
        // SAFETY: The caller guarantees the full u32 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes a `u64` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 10` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_u64_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: u64,
    ) -> usize {
        let (encoded, count) = self.u64_bytes(value);
        // SAFETY: The caller guarantees the full u64 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes a `u128` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as unsigned LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 19` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_u128_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: u128,
    ) -> usize {
        let (encoded, count) = self.u128_bytes(value);
        // SAFETY: The caller guarantees the full u128 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes an `i16` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 3` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_i16_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: i16,
    ) -> usize {
        let (encoded, count) = self.i16_bytes(value);
        // SAFETY: The caller guarantees the full i16 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes an `i32` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 5` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_i32_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: i32,
    ) -> usize {
        let (encoded, count) = self.i32_bytes(value);
        // SAFETY: The caller guarantees the full i32 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes an `i64` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 10` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_i64_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: i64,
    ) -> usize {
        let (encoded, count) = self.i64_bytes(value);
        // SAFETY: The caller guarantees the full i64 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Writes an `i128` value at `index` without checking destination bounds.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Absolute byte offset where the encoded value starts.
    /// - `value`: Value to encode as signed LEB128.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 19` is in bounds for
    /// `output`.
    #[inline]
    pub unsafe fn write_i128_at_unchecked(
        self,
        output: &mut [u8],
        index: usize,
        value: i128,
    ) -> usize {
        let (encoded, count) = self.i128_bytes(value);
        // SAFETY: The caller guarantees the full i128 destination range is valid.
        unsafe { write_encoded_at_unchecked(output, index, &encoded[..count]) }
    }

    /// Reads an unsigned LEB128 value with `bits` target width.
    fn read_uleb_at(
        self,
        input: &[u8],
        index: usize,
        bits: u32,
    ) -> Result<Option<(u128, usize)>, Leb128DecodeError> {
        if index > input.len() {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                index,
            ));
        }
        let Some((value, consumed)) = read_uleb_raw(input, index, bits)? else {
            return Ok(None);
        };
        if self.strict && !is_canonical_uleb(value, &input[index..index + consumed]) {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::NonCanonical,
                index,
            ));
        }
        Ok(Some((value, consumed)))
    }

    /// Reads a signed LEB128 value with `bits` target width.
    fn read_sleb_at(
        self,
        input: &[u8],
        index: usize,
        bits: u32,
    ) -> Result<Option<(i128, usize)>, Leb128DecodeError> {
        if index > input.len() {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                index,
            ));
        }
        let Some((value, consumed)) = read_sleb_raw(input, index, bits)? else {
            return Ok(None);
        };
        if self.strict && !is_canonical_sleb(value, &input[index..index + consumed]) {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::NonCanonical,
                index,
            ));
        }
        Ok(Some((value, consumed)))
    }

    /// Reads an unsigned LEB128 value with `bits` target width without bounds checks.
    unsafe fn read_uleb_at_unchecked(
        self,
        input: &[u8],
        index: usize,
        bits: u32,
    ) -> Result<(u128, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full maximum-width range is in bounds.
        let (value, consumed) = unsafe { read_uleb_raw_unchecked(input, index, bits)? };
        if self.strict {
            // SAFETY: `consumed` is within the caller-validated maximum-width range.
            let bytes = unsafe { bytes_at_unchecked(input, index, consumed) };
            if !is_canonical_uleb(value, bytes) {
                return Err(Leb128DecodeError::new(
                    Leb128DecodeErrorKind::NonCanonical,
                    index,
                ));
            }
        }
        Ok((value, consumed))
    }

    /// Reads a signed LEB128 value with `bits` target width without bounds checks.
    unsafe fn read_sleb_at_unchecked(
        self,
        input: &[u8],
        index: usize,
        bits: u32,
    ) -> Result<(i128, usize), Leb128DecodeError> {
        // SAFETY: The caller guarantees the full maximum-width range is in bounds.
        let (value, consumed) = unsafe { read_sleb_raw_unchecked(input, index, bits)? };
        if self.strict {
            // SAFETY: `consumed` is within the caller-validated maximum-width range.
            let bytes = unsafe { bytes_at_unchecked(input, index, consumed) };
            if !is_canonical_sleb(value, bytes) {
                return Err(Leb128DecodeError::new(
                    Leb128DecodeErrorKind::NonCanonical,
                    index,
                ));
            }
        }
        Ok((value, consumed))
    }
}

impl Default for Leb128Codec {
    /// Creates the default non-strict LEB128 codec.
    fn default() -> Self {
        Self::new()
    }
}

/// Reads an unsigned LEB128 value without canonical validation.
fn read_uleb_raw(
    input: &[u8],
    index: usize,
    bits: u32,
) -> Result<Option<(u128, usize)>, Leb128DecodeError> {
    let max_bytes = bits.div_ceil(7) as usize;
    let final_payload_bits = bits - ((max_bytes as u32 - 1) * 7);
    let max_last_payload = ((1u16 << final_payload_bits) - 1) as u8;
    let mut value = 0u128;

    for offset in 0..max_bytes {
        let byte_index = index + offset;
        let Some(&byte) = input.get(byte_index) else {
            return Ok(None);
        };
        let payload = byte & 0x7f;
        if offset == max_bytes - 1 && payload > max_last_payload {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                byte_index,
            ));
        }
        value |= (payload as u128) << (offset * 7);
        if byte & 0x80 == 0 {
            return Ok(Some((value, offset + 1)));
        }
    }
    Err(Leb128DecodeError::new(
        Leb128DecodeErrorKind::Malformed,
        index + max_bytes.saturating_sub(1),
    ))
}

/// Reads a signed LEB128 value without canonical validation.
fn read_sleb_raw(
    input: &[u8],
    index: usize,
    bits: u32,
) -> Result<Option<(i128, usize)>, Leb128DecodeError> {
    let max_bytes = bits.div_ceil(7) as usize;
    let mut value = 0i128;
    let mut shift = 0u32;

    for offset in 0..max_bytes {
        let byte_index = index + offset;
        let Some(&byte) = input.get(byte_index) else {
            return Ok(None);
        };
        let payload = byte & 0x7f;
        if is_too_wide_signed_final_payload(payload, offset as u32, bits) {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                byte_index,
            ));
        }
        value |= (payload as i128) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < i128::BITS && byte & 0x40 != 0 {
                value |= (!0i128) << shift;
            }
            return Ok(Some((value, offset + 1)));
        }
    }
    Err(Leb128DecodeError::new(
        Leb128DecodeErrorKind::Malformed,
        index + max_bytes.saturating_sub(1),
    ))
}

/// Reads an unsigned LEB128 value without bounds or canonical validation.
unsafe fn read_uleb_raw_unchecked(
    input: &[u8],
    index: usize,
    bits: u32,
) -> Result<(u128, usize), Leb128DecodeError> {
    let max_bytes = bits.div_ceil(7) as usize;
    let final_payload_bits = bits - ((max_bytes as u32 - 1) * 7);
    let max_last_payload = ((1u16 << final_payload_bits) - 1) as u8;
    let mut value = 0u128;

    for offset in 0..max_bytes {
        let byte_index = index + offset;
        // SAFETY: The caller guarantees the maximum-width range is in bounds.
        let byte = unsafe { *input.get_unchecked(byte_index) };
        let payload = byte & 0x7f;
        if offset == max_bytes - 1 && payload > max_last_payload {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                byte_index,
            ));
        }
        value |= (payload as u128) << (offset * 7);
        if byte & 0x80 == 0 {
            return Ok((value, offset + 1));
        }
    }
    Err(Leb128DecodeError::new(
        Leb128DecodeErrorKind::Malformed,
        index + max_bytes.saturating_sub(1),
    ))
}

/// Reads a signed LEB128 value without bounds or canonical validation.
unsafe fn read_sleb_raw_unchecked(
    input: &[u8],
    index: usize,
    bits: u32,
) -> Result<(i128, usize), Leb128DecodeError> {
    let max_bytes = bits.div_ceil(7) as usize;
    let mut value = 0i128;
    let mut shift = 0u32;

    for offset in 0..max_bytes {
        let byte_index = index + offset;
        // SAFETY: The caller guarantees the maximum-width range is in bounds.
        let byte = unsafe { *input.get_unchecked(byte_index) };
        let payload = byte & 0x7f;
        if is_too_wide_signed_final_payload(payload, offset as u32, bits) {
            return Err(Leb128DecodeError::new(
                Leb128DecodeErrorKind::Malformed,
                byte_index,
            ));
        }
        value |= (payload as i128) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < i128::BITS && byte & 0x40 != 0 {
                value |= (!0i128) << shift;
            }
            return Ok((value, offset + 1));
        }
    }
    Err(Leb128DecodeError::new(
        Leb128DecodeErrorKind::Malformed,
        index + max_bytes.saturating_sub(1),
    ))
}

/// Checks whether a final signed payload byte exceeds the target width.
fn is_too_wide_signed_final_payload(payload: u8, offset: u32, bits: u32) -> bool {
    let max_bytes = bits.div_ceil(7);
    if offset != max_bytes - 1 {
        return false;
    }
    let used_bits = bits - offset * 7;
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

/// Checks whether bytes are the canonical unsigned encoding for `value`.
fn is_canonical_uleb(value: u128, bytes: &[u8]) -> bool {
    let mut expected = [0_u8; MAX_LEB128_BYTES];
    let count = encode_uleb(value, &mut expected);
    &expected[..count] == bytes
}

/// Checks whether bytes are the canonical signed encoding for `value`.
fn is_canonical_sleb(value: i128, bytes: &[u8]) -> bool {
    let mut expected = [0_u8; MAX_LEB128_BYTES];
    let count = encode_sleb(value, &mut expected);
    &expected[..count] == bytes
}

/// Returns `length` bytes at `index` without checking slice bounds.
unsafe fn bytes_at_unchecked(input: &[u8], index: usize, length: usize) -> &[u8] {
    // SAFETY: The caller guarantees `index..index + length` is in bounds.
    unsafe { core::slice::from_raw_parts(input.as_ptr().add(index), length) }
}

/// Writes encoded bytes at `index`.
fn write_encoded_at(output: &mut [u8], index: usize, encoded: &[u8]) -> Option<usize> {
    if output.len().saturating_sub(index) < encoded.len() {
        return None;
    }
    output[index..index + encoded.len()].copy_from_slice(encoded);
    Some(encoded.len())
}

/// Writes encoded bytes at `index` without checking destination bounds.
unsafe fn write_encoded_at_unchecked(output: &mut [u8], index: usize, encoded: &[u8]) -> usize {
    // SAFETY: The caller guarantees the destination range is in bounds.
    let ptr = unsafe { output.as_mut_ptr().add(index) };
    // SAFETY: `encoded` and `output` do not overlap and the destination is valid.
    unsafe { ptr.copy_from_nonoverlapping(encoded.as_ptr(), encoded.len()) };
    encoded.len()
}

/// Encodes an unsigned LEB128 value into `output`.
fn encode_uleb(mut value: u128, output: &mut [u8]) -> usize {
    let mut index = 0;
    while value > 0x7f {
        output[index] = ((value as u8) & 0x7f) | 0x80;
        value >>= 7;
        index += 1;
    }
    output[index] = value as u8;
    index + 1
}

/// Encodes a signed LEB128 value into `output`.
fn encode_sleb(value: i128, output: &mut [u8]) -> usize {
    let mut remaining = value;
    let mut index = 0;
    loop {
        let byte = (remaining as u8) & 0x7f;
        remaining >>= 7;
        let is_done = (remaining == 0 && byte & 0x40 == 0) || (remaining == -1 && byte & 0x40 != 0);
        output[index] = if is_done { byte } else { byte | 0x80 };
        index += 1;
        if is_done {
            return index;
        }
    }
}
