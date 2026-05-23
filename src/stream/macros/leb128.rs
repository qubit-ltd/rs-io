/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

macro_rules! read_leb128_value {
    ($reader:expr, $ty:ty, $policy:ty, $max:expr) => {
        $reader.read_leb128::<$ty, { $max }, _>(|bytes| unsafe {
            $crate::codec::Leb128Codec::<$ty, $policy>::read_unchecked(bytes, 0)
        })
    };
}

macro_rules! write_leb128_value {
    ($writer:expr, $value:expr, $ty:ty) => {
        $writer.write_leb128::<$ty, {
            $crate::codec::Leb128Codec::<$ty, $crate::codec::NonStrict>::REQUIRED_MIN_BUFFER_LEN
        }, _>($value, |bytes, value| unsafe {
            $crate::codec::Leb128Codec::<$ty, $crate::codec::NonStrict>::write_unchecked(
                bytes, 0, value,
            )
        })
    };
}

pub(in crate::stream) use read_leb128_value;
pub(in crate::stream) use write_leb128_value;
