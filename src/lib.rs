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

mod binary_read_ext;
mod binary_write_ext;
mod buf_read_seek;
mod compare;
mod copy;
mod read_ext;
mod read_seek;
mod read_seek_ext;
mod read_write;
mod read_write_seek;
mod seek_ext;
mod var_int_read_ext;
mod var_int_write_ext;
mod wrappers;
mod write_seek;
mod write_seek_ext;

pub use binary_read_ext::BinaryReadExt;
pub use binary_write_ext::BinaryWriteExt;
pub use buf_read_seek::BufReadSeek;
pub use compare::{
    compare_content,
    content_eq,
};
pub use copy::copy_limited;
pub use read_ext::ReadExt;
pub use read_seek::ReadSeek;
pub use read_seek_ext::ReadSeekExt;
pub use read_write::ReadWrite;
pub use read_write_seek::ReadWriteSeek;
pub use seek_ext::SeekExt;
pub use var_int_read_ext::VarIntReadExt;
pub use var_int_write_ext::VarIntWriteExt;
pub use wrappers::{
    CountingReader,
    CountingWriter,
    LimitReader,
    LimitWriter,
    PositionGuard,
    TeeReader,
    TeeWriter,
};
pub use write_seek::WriteSeek;
pub use write_seek_ext::WriteSeekExt;
