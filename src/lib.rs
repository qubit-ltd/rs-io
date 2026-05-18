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

mod buf_read_seek;
mod read_ext;
mod read_seek;
mod read_seek_ext;
mod read_write;
mod read_write_seek;
mod seek_ext;
mod write_seek;
mod write_seek_ext;

pub use buf_read_seek::BufReadSeek;
pub use read_ext::ReadExt;
pub use read_seek::ReadSeek;
pub use read_seek_ext::ReadSeekExt;
pub use read_write::ReadWrite;
pub use read_write_seek::ReadWriteSeek;
pub use seek_ext::SeekExt;
pub use write_seek::WriteSeek;
pub use write_seek_ext::WriteSeekExt;
