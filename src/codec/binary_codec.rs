/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::ByteOrder;

/// A low-level codec for fixed-width binary values stored in byte buffers.
///
/// `BinaryCodec` keeps the buffer-oriented operations separate from [`ByteOrder`].
/// The byte order enum remains a pure configuration value, while this type owns
/// the responsibility of reading from and writing to byte slices.
///
/// The API has three families:
///
/// - fixed-array methods such as [`Self::read_u32_from_array`], for callers that
///   already have `[u8; N]`;
/// - checked slice methods such as [`Self::read_u32_at`] and [`Self::write_u32_at`],
///   for public boundaries and untrusted input;
/// - unchecked slice methods such as [`Self::read_u32_at_unchecked`], for hot
///   internal paths after an outer layer has already validated the range.
///
/// # Examples
///
/// ```rust
/// use qubit_io::{BinaryCodec, ByteOrder};
///
/// let codec = BinaryCodec::new(ByteOrder::BigEndian);
/// let value = codec.read_u32_from_array([0x00, 0x01, 0xf6, 0x00]);
/// assert_eq!(0x0001_f600, value);
///
/// let packet = [0xaa, 0x12, 0x34, 0xbb];
/// assert_eq!(Some(0x1234), codec.read_u16_at(&packet, 1));
/// assert_eq!(None, codec.read_u32_at(&packet, 1));
///
/// let mut output = [0_u8; 4];
/// let little_endian = BinaryCodec::new(ByteOrder::LittleEndian);
/// assert_eq!(Some(()), little_endian.write_u32_at(&mut output, 0, value));
/// assert_eq!([0x00, 0xf6, 0x01, 0x00], output);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryCodec {
    byte_order: ByteOrder,
}

impl BinaryCodec {
    /// Creates a binary codec using `byte_order`.
    ///
    /// # Parameters
    ///
    /// - `byte_order`: Byte order used by all fixed-width integer operations.
    ///
    /// # Returns
    ///
    /// Returns a codec configured with the given byte order.
    #[inline]
    pub const fn new(byte_order: ByteOrder) -> Self {
        Self { byte_order }
    }

    /// Returns the byte order used by this codec.
    #[must_use]
    #[inline]
    pub const fn byte_order(self) -> ByteOrder {
        self.byte_order
    }

    /// Updates the byte order used by this codec.
    ///
    /// # Parameters
    ///
    /// - `byte_order`: New byte order for subsequent operations.
    #[inline]
    pub fn set_byte_order(&mut self, byte_order: ByteOrder) {
        self.byte_order = byte_order;
    }

    /// Reads a `u16` value from a fixed-width byte array.
    ///
    /// Use this API when the caller already has exactly two bytes, for example
    /// after array pattern matching, `first_chunk`, or a parser that stores fixed
    /// fields as arrays. The array length is known at compile time, so converting
    /// it through `u16::from_be_bytes` or `u16::from_le_bytes` does not need slice
    /// bounds checks.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly two bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u16` value.
    #[inline]
    pub const fn read_u16_from_array(self, bytes: [u8; 2]) -> u16 {
        match self.byte_order {
            ByteOrder::BigEndian => u16::from_be_bytes(bytes),
            ByteOrder::LittleEndian => u16::from_le_bytes(bytes),
        }
    }

    /// Reads a `u32` value from a fixed-width byte array.
    ///
    /// Use this API when the caller already has exactly four bytes. It is the
    /// lowest-friction safe API for fixed protocol fields and avoids runtime
    /// slice length checks at this layer.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly four bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u32` value.
    #[inline]
    pub const fn read_u32_from_array(self, bytes: [u8; 4]) -> u32 {
        match self.byte_order {
            ByteOrder::BigEndian => u32::from_be_bytes(bytes),
            ByteOrder::LittleEndian => u32::from_le_bytes(bytes),
        }
    }

    /// Reads a `u64` value from a fixed-width byte array.
    ///
    /// Use this API when the caller already has exactly eight bytes. Like the
    /// smaller fixed-array readers, it keeps bounds responsibility outside this
    /// method.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly eight bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u64` value.
    #[inline]
    pub const fn read_u64_from_array(self, bytes: [u8; 8]) -> u64 {
        match self.byte_order {
            ByteOrder::BigEndian => u64::from_be_bytes(bytes),
            ByteOrder::LittleEndian => u64::from_le_bytes(bytes),
        }
    }

    /// Reads an `i16` value from a fixed-width byte array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly two bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i16` value.
    #[inline]
    pub const fn read_i16_from_array(self, bytes: [u8; 2]) -> i16 {
        match self.byte_order {
            ByteOrder::BigEndian => i16::from_be_bytes(bytes),
            ByteOrder::LittleEndian => i16::from_le_bytes(bytes),
        }
    }

    /// Reads an `i32` value from a fixed-width byte array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly four bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i32` value.
    #[inline]
    pub const fn read_i32_from_array(self, bytes: [u8; 4]) -> i32 {
        match self.byte_order {
            ByteOrder::BigEndian => i32::from_be_bytes(bytes),
            ByteOrder::LittleEndian => i32::from_le_bytes(bytes),
        }
    }

    /// Reads an `i64` value from a fixed-width byte array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly eight bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i64` value.
    #[inline]
    pub const fn read_i64_from_array(self, bytes: [u8; 8]) -> i64 {
        match self.byte_order {
            ByteOrder::BigEndian => i64::from_be_bytes(bytes),
            ByteOrder::LittleEndian => i64::from_le_bytes(bytes),
        }
    }

    /// Reads a `u128` value from a fixed-width byte array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly sixteen bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u128` value.
    #[inline]
    pub const fn read_u128_from_array(self, bytes: [u8; 16]) -> u128 {
        match self.byte_order {
            ByteOrder::BigEndian => u128::from_be_bytes(bytes),
            ByteOrder::LittleEndian => u128::from_le_bytes(bytes),
        }
    }

    /// Reads an `i128` value from a fixed-width byte array.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Exactly sixteen bytes in this codec's byte order.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i128` value.
    #[inline]
    pub const fn read_i128_from_array(self, bytes: [u8; 16]) -> i128 {
        match self.byte_order {
            ByteOrder::BigEndian => i128::from_be_bytes(bytes),
            ByteOrder::LittleEndian => i128::from_le_bytes(bytes),
        }
    }

    /// Reads a `u16` value at `index` from a byte slice.
    ///
    /// Use this safe API at public boundaries, while parsing untrusted input, or
    /// whenever short buffers should be represented as absence instead of panic.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when `index..index + 2` is in bounds. Returns `None`
    /// when `index` is out of bounds or fewer than two bytes remain.
    #[inline]
    pub fn read_u16_at(self, bytes: &[u8], index: usize) -> Option<u16> {
        let chunk = bytes.get(index..)?.first_chunk::<2>()?;
        Some(self.read_u16_from_array(*chunk))
    }

    /// Reads a `u32` value at `index` from a byte slice.
    ///
    /// Use this safe API when the slice length has not already been validated by
    /// a higher layer. It performs the necessary bounds check and returns `None`
    /// for incomplete values.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when `index..index + 4` is in bounds. Returns `None`
    /// when `index` is out of bounds or fewer than four bytes remain.
    #[inline]
    pub fn read_u32_at(self, bytes: &[u8], index: usize) -> Option<u32> {
        let chunk = bytes.get(index..)?.first_chunk::<4>()?;
        Some(self.read_u32_from_array(*chunk))
    }

    /// Reads a `u64` value at `index` from a byte slice.
    ///
    /// Use this safe API when parsing length-checked external buffers where an
    /// incomplete field should be handled by the caller.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when `index..index + 8` is in bounds. Returns `None`
    /// when `index` is out of bounds or fewer than eight bytes remain.
    #[inline]
    pub fn read_u64_at(self, bytes: &[u8], index: usize) -> Option<u64> {
        let chunk = bytes.get(index..)?.first_chunk::<8>()?;
        Some(self.read_u64_from_array(*chunk))
    }

    /// Reads an `i16` value at `index` from a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when the full value is available, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn read_i16_at(self, bytes: &[u8], index: usize) -> Option<i16> {
        let chunk = bytes.get(index..)?.first_chunk::<2>()?;
        Some(self.read_i16_from_array(*chunk))
    }

    /// Reads an `i32` value at `index` from a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when the full value is available, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn read_i32_at(self, bytes: &[u8], index: usize) -> Option<i32> {
        let chunk = bytes.get(index..)?.first_chunk::<4>()?;
        Some(self.read_i32_from_array(*chunk))
    }

    /// Reads an `i64` value at `index` from a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when the full value is available, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn read_i64_at(self, bytes: &[u8], index: usize) -> Option<i64> {
        let chunk = bytes.get(index..)?.first_chunk::<8>()?;
        Some(self.read_i64_from_array(*chunk))
    }

    /// Reads a `u128` value at `index` from a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when the full value is available, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn read_u128_at(self, bytes: &[u8], index: usize) -> Option<u128> {
        let chunk = bytes.get(index..)?.first_chunk::<16>()?;
        Some(self.read_u128_from_array(*chunk))
    }

    /// Reads an `i128` value at `index` from a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns `Some(value)` when the full value is available, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn read_i128_at(self, bytes: &[u8], index: usize) -> Option<i128> {
        let chunk = bytes.get(index..)?.first_chunk::<16>()?;
        Some(self.read_i128_from_array(*chunk))
    }

    /// Reads a `u16` value at `index` without checking slice bounds.
    ///
    /// Use this API only in internal hot paths where a higher layer has already
    /// checked that the full field is available. Prefer [`Self::read_u16_at`] at
    /// public or untrusted-input boundaries.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u16` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 2` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn read_u16_at_unchecked(self, bytes: &[u8], index: usize) -> u16 {
        // SAFETY: The caller guarantees that `index` starts an in-bounds two-byte range.
        let ptr = unsafe { bytes.as_ptr().add(index).cast::<[u8; 2]>() };
        // SAFETY: `[u8; 2]` has alignment 1, and the caller guarantees the range is valid.
        let chunk = unsafe { *ptr };
        self.read_u16_from_array(chunk)
    }

    /// Reads a `u32` value at `index` without checking slice bounds.
    ///
    /// Use this API inside codecs and parsers after an outer bounds check has
    /// already established that at least four bytes are available.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u32` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 4` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn read_u32_at_unchecked(self, bytes: &[u8], index: usize) -> u32 {
        // SAFETY: The caller guarantees that `index` starts an in-bounds four-byte range.
        let ptr = unsafe { bytes.as_ptr().add(index).cast::<[u8; 4]>() };
        // SAFETY: `[u8; 4]` has alignment 1, and the caller guarantees the range is valid.
        let chunk = unsafe { *ptr };
        self.read_u32_from_array(chunk)
    }

    /// Reads a `u64` value at `index` without checking slice bounds.
    ///
    /// Use this API for validated internal binary parsers that repeatedly read
    /// fixed-width fields from the same checked buffer.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u64` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 8` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn read_u64_at_unchecked(self, bytes: &[u8], index: usize) -> u64 {
        // SAFETY: The caller guarantees that `index` starts an in-bounds eight-byte range.
        let ptr = unsafe { bytes.as_ptr().add(index).cast::<[u8; 8]>() };
        // SAFETY: `[u8; 8]` has alignment 1, and the caller guarantees the range is valid.
        let chunk = unsafe { *ptr };
        self.read_u64_from_array(chunk)
    }

    /// Reads an `i16` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i16` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 2` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn read_i16_at_unchecked(self, bytes: &[u8], index: usize) -> i16 {
        // SAFETY: The caller guarantees that the two-byte range is in bounds.
        let chunk = unsafe { self.read_u16_at_unchecked(bytes, index) };
        chunk as i16
    }

    /// Reads an `i32` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i32` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 4` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn read_i32_at_unchecked(self, bytes: &[u8], index: usize) -> i32 {
        // SAFETY: The caller guarantees that the four-byte range is in bounds.
        let chunk = unsafe { self.read_u32_at_unchecked(bytes, index) };
        chunk as i32
    }

    /// Reads an `i64` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i64` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 8` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn read_i64_at_unchecked(self, bytes: &[u8], index: usize) -> i64 {
        // SAFETY: The caller guarantees that the eight-byte range is in bounds.
        let chunk = unsafe { self.read_u64_at_unchecked(bytes, index) };
        chunk as i64
    }

    /// Reads a `u128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `u128` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 16` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn read_u128_at_unchecked(self, bytes: &[u8], index: usize) -> u128 {
        // SAFETY: The caller guarantees that `index` starts an in-bounds sixteen-byte range.
        let ptr = unsafe { bytes.as_ptr().add(index).cast::<[u8; 16]>() };
        // SAFETY: `[u8; 16]` has alignment 1, and the caller guarantees the range is valid.
        let chunk = unsafe { *ptr };
        self.read_u128_from_array(chunk)
    }

    /// Reads an `i128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Source byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    ///
    /// # Returns
    ///
    /// Returns the decoded `i128` value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 16` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn read_i128_at_unchecked(self, bytes: &[u8], index: usize) -> i128 {
        // SAFETY: The caller guarantees that the sixteen-byte range is in bounds.
        let chunk = unsafe { self.read_u128_at_unchecked(bytes, index) };
        chunk as i128
    }

    /// Converts a `u16` value to bytes using this codec's byte order.
    ///
    /// Use this API when the caller wants a fixed-width array, for example before
    /// appending to a buffer or writing through an already validated destination.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns two bytes in this codec's byte order.
    #[inline]
    pub const fn u16_bytes(self, value: u16) -> [u8; 2] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts a `u32` value to bytes using this codec's byte order.
    ///
    /// Use this API when the caller wants a fixed-width array, for example before
    /// appending to a buffer or writing through an already validated destination.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns four bytes in this codec's byte order.
    #[inline]
    pub const fn u32_bytes(self, value: u32) -> [u8; 4] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts a `u64` value to bytes using this codec's byte order.
    ///
    /// Use this API when the caller wants a fixed-width array for an eight-byte
    /// integer field.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns eight bytes in this codec's byte order.
    #[inline]
    pub const fn u64_bytes(self, value: u64) -> [u8; 8] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts an `i16` value to bytes using this codec's byte order.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns two bytes in this codec's byte order.
    #[inline]
    pub const fn i16_bytes(self, value: i16) -> [u8; 2] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts an `i32` value to bytes using this codec's byte order.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns four bytes in this codec's byte order.
    #[inline]
    pub const fn i32_bytes(self, value: i32) -> [u8; 4] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts an `i64` value to bytes using this codec's byte order.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns eight bytes in this codec's byte order.
    #[inline]
    pub const fn i64_bytes(self, value: i64) -> [u8; 8] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts a `u128` value to bytes using this codec's byte order.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns sixteen bytes in this codec's byte order.
    #[inline]
    pub const fn u128_bytes(self, value: u128) -> [u8; 16] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Converts an `i128` value to bytes using this codec's byte order.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns sixteen bytes in this codec's byte order.
    #[inline]
    pub const fn i128_bytes(self, value: i128) -> [u8; 16] {
        match self.byte_order {
            ByteOrder::BigEndian => value.to_be_bytes(),
            ByteOrder::LittleEndian => value.to_le_bytes(),
        }
    }

    /// Writes a `u16` value at `index` into a byte slice.
    ///
    /// Use this safe API at public boundaries or when destination capacity has
    /// not already been checked.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when `index..index + 2` is in bounds. Returns `None`
    /// when the destination does not have enough space.
    #[inline]
    pub fn write_u16_at(self, bytes: &mut [u8], index: usize, value: u16) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<2>()?;
        *chunk = self.u16_bytes(value);
        Some(())
    }

    /// Writes a `u32` value at `index` into a byte slice.
    ///
    /// Use this safe API when a short destination should be reported to the caller
    /// instead of causing a panic.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when `index..index + 4` is in bounds. Returns `None`
    /// when the destination does not have enough space.
    #[inline]
    pub fn write_u32_at(self, bytes: &mut [u8], index: usize, value: u32) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<4>()?;
        *chunk = self.u32_bytes(value);
        Some(())
    }

    /// Writes a `u64` value at `index` into a byte slice.
    ///
    /// Use this safe API for externally supplied or dynamically sized
    /// destinations where capacity should be checked locally.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when `index..index + 8` is in bounds. Returns `None`
    /// when the destination does not have enough space.
    #[inline]
    pub fn write_u64_at(self, bytes: &mut [u8], index: usize, value: u64) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<8>()?;
        *chunk = self.u64_bytes(value);
        Some(())
    }

    /// Writes an `i16` value at `index` into a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when the destination has enough space, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn write_i16_at(self, bytes: &mut [u8], index: usize, value: i16) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<2>()?;
        *chunk = self.i16_bytes(value);
        Some(())
    }

    /// Writes an `i32` value at `index` into a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when the destination has enough space, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn write_i32_at(self, bytes: &mut [u8], index: usize, value: i32) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<4>()?;
        *chunk = self.i32_bytes(value);
        Some(())
    }

    /// Writes an `i64` value at `index` into a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when the destination has enough space, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn write_i64_at(self, bytes: &mut [u8], index: usize, value: i64) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<8>()?;
        *chunk = self.i64_bytes(value);
        Some(())
    }

    /// Writes a `u128` value at `index` into a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when the destination has enough space, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn write_u128_at(self, bytes: &mut [u8], index: usize, value: u128) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<16>()?;
        *chunk = self.u128_bytes(value);
        Some(())
    }

    /// Writes an `i128` value at `index` into a byte slice.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Returns
    ///
    /// Returns `Some(())` when the destination has enough space, or `None` when
    /// the byte range is out of bounds.
    #[inline]
    pub fn write_i128_at(self, bytes: &mut [u8], index: usize, value: i128) -> Option<()> {
        let chunk = bytes.get_mut(index..)?.first_chunk_mut::<16>()?;
        *chunk = self.i128_bytes(value);
        Some(())
    }

    /// Writes a `u16` value at `index` without checking slice bounds.
    ///
    /// Use this API only after an outer layer has checked that two bytes are
    /// available from `index`. Prefer [`Self::write_u16_at`] at public boundaries.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 2` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn write_u16_at_unchecked(self, bytes: &mut [u8], index: usize, value: u16) {
        let value = self.u16_bytes(value);
        // SAFETY: The caller guarantees that `index` starts an in-bounds two-byte range.
        let ptr = unsafe { bytes.as_mut_ptr().add(index) };
        // SAFETY: `value` is a distinct stack array and the destination range is valid.
        unsafe { ptr.copy_from_nonoverlapping(value.as_ptr(), 2) };
    }

    /// Writes a `u32` value at `index` without checking slice bounds.
    ///
    /// Use this API in validated internal paths where the destination range has
    /// already been checked once by the caller.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 4` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn write_u32_at_unchecked(self, bytes: &mut [u8], index: usize, value: u32) {
        let value = self.u32_bytes(value);
        // SAFETY: The caller guarantees that `index` starts an in-bounds four-byte range.
        let ptr = unsafe { bytes.as_mut_ptr().add(index) };
        // SAFETY: `value` is a distinct stack array and the destination range is valid.
        unsafe { ptr.copy_from_nonoverlapping(value.as_ptr(), 4) };
    }

    /// Writes a `u64` value at `index` without checking slice bounds.
    ///
    /// Use this API in hot binary codecs after the destination buffer has already
    /// been validated by an outer loop.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 8` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn write_u64_at_unchecked(self, bytes: &mut [u8], index: usize, value: u64) {
        let value = self.u64_bytes(value);
        // SAFETY: The caller guarantees that `index` starts an in-bounds eight-byte range.
        let ptr = unsafe { bytes.as_mut_ptr().add(index) };
        // SAFETY: `value` is a distinct stack array and the destination range is valid.
        unsafe { ptr.copy_from_nonoverlapping(value.as_ptr(), 8) };
    }

    /// Writes an `i16` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the two-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 2` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn write_i16_at_unchecked(self, bytes: &mut [u8], index: usize, value: i16) {
        // SAFETY: The caller guarantees that the two-byte range is in bounds.
        unsafe { self.write_u16_at_unchecked(bytes, index, value as u16) };
    }

    /// Writes an `i32` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the four-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 4` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline(always)]
    pub unsafe fn write_i32_at_unchecked(self, bytes: &mut [u8], index: usize, value: i32) {
        // SAFETY: The caller guarantees that the four-byte range is in bounds.
        unsafe { self.write_u32_at_unchecked(bytes, index, value as u32) };
    }

    /// Writes an `i64` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the eight-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 8` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn write_i64_at_unchecked(self, bytes: &mut [u8], index: usize, value: i64) {
        // SAFETY: The caller guarantees that the eight-byte range is in bounds.
        unsafe { self.write_u64_at_unchecked(bytes, index, value as u64) };
    }

    /// Writes a `u128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 16` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn write_u128_at_unchecked(self, bytes: &mut [u8], index: usize, value: u128) {
        let value = self.u128_bytes(value);
        // SAFETY: The caller guarantees that `index` starts an in-bounds sixteen-byte range.
        let ptr = unsafe { bytes.as_mut_ptr().add(index) };
        // SAFETY: `value` is a distinct stack array and the destination range is valid.
        unsafe { ptr.copy_from_nonoverlapping(value.as_ptr(), 16) };
    }

    /// Writes an `i128` value at `index` without checking slice bounds.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Destination byte slice.
    /// - `index`: Absolute byte offset where the sixteen-byte value starts.
    /// - `value`: The value to serialize.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + 16` is in bounds for
    /// `bytes`. Violating this requirement is undefined behavior.
    #[inline]
    pub unsafe fn write_i128_at_unchecked(self, bytes: &mut [u8], index: usize, value: i128) {
        // SAFETY: The caller guarantees that the sixteen-byte range is in bounds.
        unsafe { self.write_u128_at_unchecked(bytes, index, value as u128) };
    }
}
