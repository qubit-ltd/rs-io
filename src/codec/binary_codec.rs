/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use core::{
    marker::PhantomData,
    ptr,
};

use crate::{
    BigEndian,
    LittleEndian,
};

/// Type-level unchecked binary codec for one scalar type and one byte order.
///
/// `BinaryCodec` is intentionally a static namespace. It does not provide safe
/// checked helpers, constructors, or instance methods. Callers must validate
/// buffer lengths before entering the hot path.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BinaryCodec<T, O> {
    marker: PhantomData<fn() -> (T, O)>,
}

impl<O> BinaryCodec<u8, O> {
    /// Minimum number of bytes required to encode or decode this type.
    pub const REQUIRED_MIN_BUFFER_LEN: usize = 1;

    /// Decodes a value from `input` starting at `index` without bounds checks.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte buffer.
    /// - `index`: Start index in `input`.
    ///
    /// # Returns
    ///
    /// Returns the decoded value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input.as_ptr().add(index)` is valid to
    /// read [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes.
    #[must_use]
    #[inline(always)]
    pub unsafe fn read_unchecked(input: &[u8], index: usize) -> u8 {
        // SAFETY: The caller guarantees that the indexed byte is readable.
        unsafe { *input.as_ptr().add(index) }
    }

    /// Encodes `value` into `output` starting at `index` without bounds checks.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte buffer.
    /// - `index`: Start index in `output`.
    /// - `value`: Value to encode.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output.as_mut_ptr().add(index)` is valid
    /// to write [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes.
    #[inline(always)]
    pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: u8) {
        // SAFETY: The caller guarantees that the indexed byte is writable.
        unsafe {
            *output.as_mut_ptr().add(index) = value;
        }
    }
}

impl<O> BinaryCodec<i8, O> {
    /// Minimum number of bytes required to encode or decode this type.
    pub const REQUIRED_MIN_BUFFER_LEN: usize = 1;

    /// Decodes a value from `input` starting at `index` without bounds checks.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte buffer.
    /// - `index`: Start index in `input`.
    ///
    /// # Returns
    ///
    /// Returns the decoded value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `input.as_ptr().add(index)` is valid to
    /// read [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes.
    #[must_use]
    #[inline(always)]
    pub unsafe fn read_unchecked(input: &[u8], index: usize) -> i8 {
        // SAFETY: The caller guarantees that the indexed byte is readable.
        unsafe { *input.as_ptr().add(index) as i8 }
    }

    /// Encodes `value` into `output` starting at `index` without bounds checks.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte buffer.
    /// - `index`: Start index in `output`.
    /// - `value`: Value to encode.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `output.as_mut_ptr().add(index)` is valid
    /// to write [`Self::REQUIRED_MIN_BUFFER_LEN`] bytes.
    #[inline(always)]
    pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: i8) {
        // SAFETY: The caller guarantees that the indexed byte is writable.
        unsafe {
            *output.as_mut_ptr().add(index) = value as u8;
        }
    }
}

macro_rules! impl_integer_binary_codec {
    ($ty:ty, $len:expr) => {
        impl BinaryCodec<$ty, BigEndian> {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = $len;

            /// Decodes a value from `input` starting at `index` without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start byte index in `input`.
            ///
            /// # Returns
            ///
            /// Returns the decoded value.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len()`
            /// - `input[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for reading.
            #[must_use]
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> $ty {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len());

                // SAFETY:
                // The caller guarantees that the readable range is fully in-bounds.
                // `read_unaligned` permits unaligned memory access.
                let pointer = unsafe { input.as_ptr().add(index).cast::<$ty>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer load.
                let raw = unsafe { ptr::read_unaligned(pointer) };

                <$ty>::from_be(raw)
            }

            /// Encodes `value` into `output` starting at `index`
            /// without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `output`: Destination byte buffer.
            /// - `index`: Start byte index in `output`.
            /// - `value`: Value to encode.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len()`
            /// - `output[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for writing.
            #[inline(always)]
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len());

                let raw = value.to_be();

                // SAFETY:
                // The caller guarantees that the writable range is fully in-bounds.
                // `write_unaligned` permits unaligned memory access.
                let pointer = unsafe { output.as_mut_ptr().add(index).cast::<$ty>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer store.
                unsafe {
                    ptr::write_unaligned(pointer, raw);
                }
            }
        }

        impl BinaryCodec<$ty, LittleEndian> {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = $len;

            /// Decodes a value from `input` starting at `index` without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start byte index in `input`.
            ///
            /// # Returns
            ///
            /// Returns the decoded value.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len()`
            /// - `input[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for reading.
            #[must_use]
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> $ty {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len());

                // SAFETY:
                // The caller guarantees that the readable range is fully in-bounds.
                // `read_unaligned` permits unaligned memory access.
                let pointer = unsafe { input.as_ptr().add(index).cast::<$ty>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer load.
                let raw = unsafe { ptr::read_unaligned(pointer) };

                <$ty>::from_le(raw)
            }

            /// Encodes `value` into `output` starting at `index`
            /// without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `output`: Destination byte buffer.
            /// - `index`: Start byte index in `output`.
            /// - `value`: Value to encode.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len()`
            /// - `output[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for writing.
            #[inline(always)]
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len());

                let raw = value.to_le();

                // SAFETY:
                // The caller guarantees that the writable range is fully in-bounds.
                // `write_unaligned` permits unaligned memory access.
                let pointer = unsafe { output.as_mut_ptr().add(index).cast::<$ty>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer store.
                unsafe {
                    ptr::write_unaligned(pointer, raw);
                }
            }
        }
    };
}

macro_rules! impl_float_binary_codec {
    ($ty:ty, $bits:ty, $len:expr) => {
        impl BinaryCodec<$ty, BigEndian> {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = $len;

            /// Decodes a value from `input` starting at `index` without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start byte index in `input`.
            ///
            /// # Returns
            ///
            /// Returns the decoded floating-point value.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len()`
            /// - `input[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for reading.
            #[must_use]
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> $ty {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len());

                // SAFETY:
                // The caller guarantees that the readable range is fully in-bounds.
                // `read_unaligned` permits unaligned memory access.
                let pointer = unsafe { input.as_ptr().add(index).cast::<$bits>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer load.
                let raw = unsafe { ptr::read_unaligned(pointer) };

                <$ty>::from_bits(<$bits>::from_be(raw))
            }

            /// Encodes `value` into `output` starting at `index`
            /// without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `output`: Destination byte buffer.
            /// - `index`: Start byte index in `output`.
            /// - `value`: Floating-point value to encode.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len()`
            /// - `output[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for writing.
            #[inline(always)]
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len());

                let raw = value.to_bits().to_be();

                // SAFETY:
                // The caller guarantees that the writable range is fully in-bounds.
                // `write_unaligned` permits unaligned memory access.
                let pointer = unsafe { output.as_mut_ptr().add(index).cast::<$bits>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer store.
                unsafe {
                    ptr::write_unaligned(pointer, raw);
                }
            }
        }

        impl BinaryCodec<$ty, LittleEndian> {
            /// Minimum number of bytes required to encode or decode this type.
            pub const REQUIRED_MIN_BUFFER_LEN: usize = $len;

            /// Decodes a value from `input` starting at `index` without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `input`: Source byte buffer.
            /// - `index`: Start byte index in `input`.
            ///
            /// # Returns
            ///
            /// Returns the decoded floating-point value.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len()`
            /// - `input[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for reading.
            #[must_use]
            #[inline(always)]
            pub unsafe fn read_unchecked(input: &[u8], index: usize) -> $ty {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= input.len());

                // SAFETY:
                // The caller guarantees that the readable range is fully in-bounds.
                // `read_unaligned` permits unaligned memory access.
                let pointer = unsafe { input.as_ptr().add(index).cast::<$bits>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer load.
                let raw = unsafe { ptr::read_unaligned(pointer) };

                <$ty>::from_bits(<$bits>::from_le(raw))
            }

            /// Encodes `value` into `output` starting at `index`
            /// without bounds checks.
            ///
            /// This function is intended for hot binary codec paths where the
            /// caller has already validated the buffer length externally.
            ///
            /// # Parameters
            ///
            /// - `output`: Destination byte buffer.
            /// - `index`: Start byte index in `output`.
            /// - `value`: Floating-point value to encode.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that:
            ///
            /// - `index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len()`
            /// - `output[index..index + Self::REQUIRED_MIN_BUFFER_LEN]`
            ///   is valid for writing.
            #[inline(always)]
            pub unsafe fn write_unchecked(output: &mut [u8], index: usize, value: $ty) {
                debug_assert!(index + Self::REQUIRED_MIN_BUFFER_LEN <= output.len());

                let raw = value.to_bits().to_le();

                // SAFETY:
                // The caller guarantees that the writable range is fully in-bounds.
                // `write_unaligned` permits unaligned memory access.
                let pointer = unsafe { output.as_mut_ptr().add(index).cast::<$bits>() };

                // SAFETY:
                // The pointer is valid for an unaligned integer store.
                unsafe {
                    ptr::write_unaligned(pointer, raw);
                }
            }
        }
    };
}

impl_integer_binary_codec!(u16, 2);
impl_integer_binary_codec!(u32, 4);
impl_integer_binary_codec!(u64, 8);
impl_integer_binary_codec!(u128, 16);
impl_integer_binary_codec!(i16, 2);
impl_integer_binary_codec!(i32, 4);
impl_integer_binary_codec!(i64, 8);
impl_integer_binary_codec!(i128, 16);
impl_float_binary_codec!(f32, u32, 4);
impl_float_binary_codec!(f64, u64, 8);
