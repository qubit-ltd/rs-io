/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
mod allocation;
mod streams;

pub(crate) use allocation::{
    try_reserve_string,
    try_reserve_vec,
};
pub use streams::Streams;
pub(crate) use streams::{
    checked_u16_len,
    checked_u32_len,
    read_leb128_payload,
    read_utf8_payload,
    write_utf8_payload,
    write_utf8_string_with_u16_len,
    write_utf8_string_with_u32_len,
};
