/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

macro_rules! impl_binary_reader_for_order {
    ($order:ty) => {
        impl<R> $crate::stream::BinaryReader<R, $order>
        where
            R: ::std::io::Read,
        {
            /// Reads an unsigned 8-bit integer.
            #[inline]
            pub fn read_u8(&mut self) -> ::std::io::Result<u8> {
                self.read_binary_with::<u8, { $crate::codec::BinaryCodec::<u8, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<u8, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a signed 8-bit integer.
            #[inline]
            pub fn read_i8(&mut self) -> ::std::io::Result<i8> {
                self.read_binary_with::<i8, { $crate::codec::BinaryCodec::<i8, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<i8, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads an unsigned 16-bit integer.
            #[inline]
            pub fn read_u16(&mut self) -> ::std::io::Result<u16> {
                self.read_binary_with::<u16, { $crate::codec::BinaryCodec::<u16, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<u16, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads an unsigned 32-bit integer.
            #[inline]
            pub fn read_u32(&mut self) -> ::std::io::Result<u32> {
                self.read_binary_with::<u32, { $crate::codec::BinaryCodec::<u32, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<u32, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads an unsigned 64-bit integer.
            #[inline]
            pub fn read_u64(&mut self) -> ::std::io::Result<u64> {
                self.read_binary_with::<u64, { $crate::codec::BinaryCodec::<u64, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<u64, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads an unsigned 128-bit integer.
            #[inline]
            pub fn read_u128(&mut self) -> ::std::io::Result<u128> {
                self.read_binary_with::<u128, { $crate::codec::BinaryCodec::<u128, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<u128, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a signed 16-bit integer.
            #[inline]
            pub fn read_i16(&mut self) -> ::std::io::Result<i16> {
                self.read_binary_with::<i16, { $crate::codec::BinaryCodec::<i16, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<i16, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a signed 32-bit integer.
            #[inline]
            pub fn read_i32(&mut self) -> ::std::io::Result<i32> {
                self.read_binary_with::<i32, { $crate::codec::BinaryCodec::<i32, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<i32, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a signed 64-bit integer.
            #[inline]
            pub fn read_i64(&mut self) -> ::std::io::Result<i64> {
                self.read_binary_with::<i64, { $crate::codec::BinaryCodec::<i64, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<i64, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a signed 128-bit integer.
            #[inline]
            pub fn read_i128(&mut self) -> ::std::io::Result<i128> {
                self.read_binary_with::<i128, { $crate::codec::BinaryCodec::<i128, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<i128, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a 32-bit float.
            #[inline]
            pub fn read_f32(&mut self) -> ::std::io::Result<f32> {
                self.read_binary_with::<f32, { $crate::codec::BinaryCodec::<f32, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<f32, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a 64-bit float.
            #[inline]
            pub fn read_f64(&mut self) -> ::std::io::Result<f64> {
                self.read_binary_with::<f64, { $crate::codec::BinaryCodec::<f64, $order>::REQUIRED_MIN_BUFFER_LEN }, _>(
                    |bytes| unsafe { $crate::codec::BinaryCodec::<f64, $order>::read_unchecked(bytes, 0) },
                )
            }

            /// Reads a UTF-8 string prefixed by a 16-bit byte length.
            ///
            /// # Parameters
            ///
            /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
            ///
            /// # Errors
            ///
            /// Returns [`std::io::ErrorKind::InvalidData`] when the encoded length exceeds
            /// `max_len` or when the payload is not valid UTF-8.
            #[inline]
            pub fn read_utf8_string_u16(&mut self, max_len: usize) -> ::std::io::Result<String> {
                let len = usize::from(self.read_u16()?);
                $crate::util::read_utf8_payload(&mut self.inner, len, max_len)
            }

            /// Reads a UTF-8 string prefixed by a 32-bit byte length.
            ///
            /// # Parameters
            ///
            /// - `max_len`: Maximum accepted UTF-8 payload length in bytes.
            ///
            /// # Errors
            ///
            /// Returns [`std::io::ErrorKind::InvalidData`] when the encoded length exceeds
            /// `max_len` or when the payload is not valid UTF-8.
            #[inline]
            pub fn read_utf8_string_u32(&mut self, max_len: usize) -> ::std::io::Result<String> {
                let len = self.read_u32()? as usize;
                $crate::util::read_utf8_payload(&mut self.inner, len, max_len)
            }
        }
    };
}

macro_rules! impl_binary_writer_for_order {
    ($order:ty) => {
        impl<W> $crate::stream::BinaryWriter<W, $order>
        where
            W: ::std::io::Write,
        {
            /// Writes an unsigned 8-bit integer.
            #[inline]
            pub fn write_u8(&mut self, value: u8) -> ::std::io::Result<()> {
                self.write_binary::<
                    u8,
                    { $crate::codec::BinaryCodec::<u8, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<u8, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a signed 8-bit integer.
            #[inline]
            pub fn write_i8(&mut self, value: i8) -> ::std::io::Result<()> {
                self.write_binary::<
                    i8,
                    { $crate::codec::BinaryCodec::<i8, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<i8, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes an unsigned 16-bit integer.
            #[inline]
            pub fn write_u16(&mut self, value: u16) -> ::std::io::Result<()> {
                self.write_binary::<
                    u16,
                    { $crate::codec::BinaryCodec::<u16, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<u16, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes an unsigned 32-bit integer.
            #[inline]
            pub fn write_u32(&mut self, value: u32) -> ::std::io::Result<()> {
                self.write_binary::<
                    u32,
                    { $crate::codec::BinaryCodec::<u32, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<u32, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes an unsigned 64-bit integer.
            #[inline]
            pub fn write_u64(&mut self, value: u64) -> ::std::io::Result<()> {
                self.write_binary::<
                    u64,
                    { $crate::codec::BinaryCodec::<u64, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<u64, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes an unsigned 128-bit integer.
            #[inline]
            pub fn write_u128(&mut self, value: u128) -> ::std::io::Result<()> {
                self.write_binary::<
                    u128,
                    { $crate::codec::BinaryCodec::<u128, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<u128, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a signed 16-bit integer.
            #[inline]
            pub fn write_i16(&mut self, value: i16) -> ::std::io::Result<()> {
                self.write_binary::<
                    i16,
                    { $crate::codec::BinaryCodec::<i16, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<i16, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a signed 32-bit integer.
            #[inline]
            pub fn write_i32(&mut self, value: i32) -> ::std::io::Result<()> {
                self.write_binary::<
                    i32,
                    { $crate::codec::BinaryCodec::<i32, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<i32, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a signed 64-bit integer.
            #[inline]
            pub fn write_i64(&mut self, value: i64) -> ::std::io::Result<()> {
                self.write_binary::<
                    i64,
                    { $crate::codec::BinaryCodec::<i64, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<i64, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a signed 128-bit integer.
            #[inline]
            pub fn write_i128(&mut self, value: i128) -> ::std::io::Result<()> {
                self.write_binary::<
                    i128,
                    { $crate::codec::BinaryCodec::<i128, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<i128, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a 32-bit float.
            #[inline]
            pub fn write_f32(&mut self, value: f32) -> ::std::io::Result<()> {
                self.write_binary::<
                    f32,
                    { $crate::codec::BinaryCodec::<f32, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<f32, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a 64-bit float.
            #[inline]
            pub fn write_f64(&mut self, value: f64) -> ::std::io::Result<()> {
                self.write_binary::<
                    f64,
                    { $crate::codec::BinaryCodec::<f64, $order>::REQUIRED_MIN_BUFFER_LEN },
                    _,
                >(value, |buffer, value| unsafe {
                    $crate::codec::BinaryCodec::<f64, $order>::write_unchecked(buffer, 0, value)
                })
            }

            /// Writes a UTF-8 string prefixed by a 16-bit byte length.
            #[inline]
            pub fn write_utf8_string_u16(&mut self, value: &str) -> ::std::io::Result<()> {
                self.write_u16($crate::stream::binary_writer::checked_u16_len(value.len())?)?;
                self.inner.write_all(value.as_bytes())
            }

            /// Writes a UTF-8 string prefixed by a 32-bit byte length.
            #[inline]
            pub fn write_utf8_string_u32(&mut self, value: &str) -> ::std::io::Result<()> {
                self.write_u32($crate::stream::binary_writer::checked_u32_len(value.len())?)?;
                self.inner.write_all(value.as_bytes())
            }
        }
    };
}

pub(in crate::stream) use impl_binary_reader_for_order;
pub(in crate::stream) use impl_binary_writer_for_order;
