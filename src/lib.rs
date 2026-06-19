// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! # Qubit IO
//!
//! Unit-oriented buffering and small I/O trait utilities for Rust.
//!
//! This crate provides named, object-safe composition traits for common
//! [`std::io`] capability combinations and small extension traits for recurring
//! standard-library I/O patterns.
//!
//! It also provides item-oriented buffering primitives in [`buffered`]:
//! [`Buffer`], [`BufferedInput`], and [`BufferedOutput`]. These types
//! are intentionally format-agnostic. Binary and text stream adapters live in
//! sibling crates and build their codec-specific behavior on top of these item
//! windows.
//!
//! The concrete trait definitions and wrapper types live in dedicated modules
//! and are re-exported from the crate root for ergonomic use.

pub mod buffered;
mod capacity_const;
pub mod ext;
mod traits;
mod util;
mod wrappers;

pub use buffered::{
    Buffer,
    BufferedInput,
    BufferedOutput,
};
pub use capacity_const::DEFAULT_BUFFER_CAPACITY;
pub use ext::{
    BufReadExt,
    InputExt,
    OutputExt,
    ReadExt,
    ReadSeekExt,
    SeekExt,
    WriteExt,
    WriteSeekExt,
};
pub use traits::{
    BufReadSeek,
    Input,
    Output,
    ReadSeek,
    ReadWrite,
    ReadWriteSeek,
    Seekable,
    SeekableInput,
    SeekableOutput,
    WriteSeek,
};
pub use util::Streams;
#[allow(unused_imports)]
pub use util::UncheckedSlice;
#[allow(unused_imports)]
pub use util::{
    nz,
    nz_const,
};
pub use util::{
    try_reserve_string,
    try_reserve_vec,
};
pub use wrappers::{
    ChecksumReader,
    ChecksumWriter,
    CountingReader,
    CountingWriter,
    LimitReader,
    LimitWriter,
    PositionGuard,
    SyncSeekTeeReader,
    TeeReader,
    TeeWriter,
};
