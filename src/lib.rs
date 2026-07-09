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
//! [`Buffer`], [`BufferedInput`], [`BufferedOutput`],
//! [`EnsuredBufferedInput`], and [`EnsuredBufferedOutput`]. These types are
//! intentionally format-agnostic. Binary and text stream adapters live in
//! sibling crates and build their codec-specific behavior on top of these item
//! windows.
//!
//! The concrete trait definitions and wrapper types live in dedicated modules
//! and are re-exported from the crate root for ergonomic use.
// qubit-style: allow coverage-cfg

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
    EnsuredBufferedInput,
    EnsuredBufferedOutput,
};
pub use capacity_const::{
    DEFAULT_BUFFER_CAPACITY,
    DEFAULT_COMPARE_BUFFER_SIZE,
    DEFAULT_COPY_BUFFER_SIZE,
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
#[cfg(coverage)]
#[doc(hidden)]
pub use util::{
    coverage_add_item_count_overflow,
    coverage_fail_next_add_item_count,
    coverage_fail_next_reserve,
    coverage_fail_next_string_reserve,
    coverage_fail_reserve_after,
    coverage_reset_add_item_count_hooks,
    coverage_reset_reserve_hooks,
};
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
