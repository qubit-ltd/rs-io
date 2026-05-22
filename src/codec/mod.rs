/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0.
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Generic, charset-independent coding traits and status types.

mod binary_codec;
mod byte_order;
mod leb128_codec;
mod leb128_decode_error;
mod leb128_decode_error_kind;
mod zig_zag_codec;

pub use binary_codec::BinaryCodec;
pub use byte_order::ByteOrder;
pub use leb128_codec::Leb128Codec;
pub use leb128_decode_error::Leb128DecodeError;
pub use leb128_decode_error_kind::Leb128DecodeErrorKind;
pub use zig_zag_codec::ZigZagCodec;
