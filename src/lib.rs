/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Qubit IO
//!
//! Small I/O trait utilities for Rust.
//!
//! This crate provides named, object-safe composition traits for common
//! [`std::io`] capability combinations and small extension traits for recurring
//! standard-library I/O patterns. The concrete trait definitions live in
//! dedicated modules and are re-exported from the crate root for ergonomic use.

// qubit-style: allow coverage-cfg
mod ext;
mod traits;
mod util;
mod wrappers;

#[cfg(coverage)]
pub use ext::string_write_ext::coverage_checked_u32_len;
pub use ext::{
    BinaryReadExt,
    BinaryWriteExt,
    BufReadExt,
    ByteOrder,
    Leb128IntReadExt,
    Leb128IntWriteExt,
    ReadExt,
    ReadSeekExt,
    SeekExt,
    StringReadExt,
    StringWriteExt,
    WriteSeekExt,
    ZigZagIntReadExt,
    ZigZagIntWriteExt,
};
pub use traits::{
    BufReadSeek,
    ReadSeek,
    ReadWrite,
    ReadWriteSeek,
    WriteSeek,
};
#[cfg(coverage)]
pub use util::file::coverage_exercise_file_helper_defensive_paths;
pub use util::{
    atomic_write,
    atomic_write_with,
    compare_content,
    content_eq,
    copy_limited,
    create_buffered_writer_with_parent,
    create_file_with_parent,
    open_buffered_reader,
};
pub use wrappers::{
    ChecksumReader,
    ChecksumWriter,
    CountingReader,
    CountingWriter,
    LimitReader,
    LimitWriter,
    PositionGuard,
    TeeReader,
    TeeWriter,
};
