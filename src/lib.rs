// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! # Qubit IO
//!
//! Byte-stream buffering and small I/O trait utilities for Rust.
//!
//! This crate provides named, object-safe composition traits for common
//! [`std::io`] capability combinations and small extension traits for recurring
//! standard-library I/O patterns.
//!
//! It also provides byte-oriented buffering primitives in [`buffered`]:
//! [`Buffer`], [`BufferedByteInput`], and [`BufferedByteOutput`]. These types
//! are intentionally format-agnostic. Binary and text stream adapters live in
//! sibling crates and build their codec-specific behavior on top of these byte
//! windows.
//!
//! The concrete trait definitions and wrapper types live in dedicated modules
//! and are re-exported from the crate root for ergonomic use.

pub mod buffered;
mod ext;
pub mod prelude;
mod traits;
mod util;
mod wrappers;

pub use buffered::{
    Buffer,
    BufferedByteInput,
    BufferedByteOutput,
    DEFAULT_BUFFER_CAPACITY,
};
pub use ext::{
    BufReadExt,
    ReadExt,
    ReadSeekExt,
    SeekExt,
    WriteExt,
    WriteSeekExt,
};
pub use traits::{
    BufReadSeek,
    ReadSeek,
    ReadWrite,
    ReadWriteSeek,
    WriteSeek,
};
pub use util::Streams;
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
