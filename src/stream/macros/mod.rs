/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Internal macros for stream reader and writer implementations.

mod binary;
mod leb128;
mod zig_zag;

pub(in crate::stream) use binary::{impl_binary_reader_for_order, impl_binary_writer_for_order};
pub(in crate::stream) use leb128::{read_leb128_value, write_leb128_value};
pub(in crate::stream) use zig_zag::{read_zig_zag_value, write_zig_zag_value};
