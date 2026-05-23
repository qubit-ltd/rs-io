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
mod buffered_binary_reader;
mod buffered_binary_writer;
mod buffered_input;
mod buffered_leb128_reader;
mod buffered_leb128_writer;
mod buffered_output;
mod buffered_zig_zag_reader;
mod buffered_zig_zag_writer;
mod leb128_reader;
mod leb128_writer;
mod zig_zag_reader;
mod zig_zag_writer;

pub use binary_reader::BinaryReader;
pub use binary_writer::BinaryWriter;
pub use buffered_binary_reader::BufferedBinaryReader;
pub use buffered_binary_writer::BufferedBinaryWriter;
pub(crate) use buffered_input::BufferedInput;
pub use buffered_leb128_reader::BufferedLeb128Reader;
pub use buffered_leb128_writer::BufferedLeb128Writer;
pub(crate) use buffered_output::BufferedOutput;
pub use buffered_zig_zag_reader::BufferedZigZagReader;
pub use buffered_zig_zag_writer::BufferedZigZagWriter;
pub use leb128_reader::Leb128Reader;
pub use leb128_writer::Leb128Writer;
pub use zig_zag_reader::ZigZagReader;
pub use zig_zag_writer::ZigZagWriter;
