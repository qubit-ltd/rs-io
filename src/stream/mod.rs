/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Reader and writer wrapper types for codec-oriented I/O.

mod binary_reader;
mod binary_writer;
mod leb128_reader;
mod leb128_writer;
mod zig_zag_reader;
mod zig_zag_writer;

pub use binary_reader::BinaryReader;
pub use binary_writer::BinaryWriter;
pub use leb128_reader::Leb128Reader;
pub use leb128_writer::Leb128Writer;
pub use zig_zag_reader::ZigZagReader;
pub use zig_zag_writer::ZigZagWriter;
