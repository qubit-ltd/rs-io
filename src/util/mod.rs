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
pub(crate) use streams::read_leb128_payload;
