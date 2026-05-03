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
//! [`std::io`] capability combinations. The concrete trait definitions live in
//! dedicated modules and are re-exported from the crate root for ergonomic use.

mod read_seek;
mod read_write;
mod read_write_seek;
mod write_seek;

pub use read_seek::ReadSeek;
pub use read_write::ReadWrite;
pub use read_write_seek::ReadWriteSeek;
pub use write_seek::WriteSeek;
