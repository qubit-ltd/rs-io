/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

macro_rules! read_zig_zag_value {
    ($reader:expr, $ty:ty, $policy:ty) => {
        $reader.read_leb128::<$ty, {
            $crate::codec::ZigZagCodec::<$ty, $crate::codec::NonStrict>::REQUIRED_MIN_BUFFER_LEN
        }, _>(|bytes| unsafe {
            $crate::codec::ZigZagCodec::<$ty, $policy>::read_unchecked(bytes, 0)
        })
    };
}

macro_rules! write_zig_zag_value {
    ($writer:expr, $value:expr, $ty:ty) => {
        $writer.write_zig_zag::<$ty, {
            $crate::codec::ZigZagCodec::<$ty, $crate::codec::NonStrict>::REQUIRED_MIN_BUFFER_LEN
        }, _>($value, |bytes, value| unsafe {
            $crate::codec::ZigZagCodec::<$ty, $crate::codec::NonStrict>::write_unchecked(
                bytes, 0, value,
            )
        })
    };
}

pub(in crate::stream) use read_zig_zag_value;
pub(in crate::stream) use write_zig_zag_value;
