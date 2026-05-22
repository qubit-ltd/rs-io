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

mod codec;
mod ext;
pub mod prelude;
mod stream;
mod traits;
mod util;
mod wrappers;

pub use codec::{
    BinaryCodec,
    Coder,
    CoderProgress,
    CoderStatus,
    Leb128Codec,
    Leb128DecodeError,
    Leb128DecodeErrorKind,
    ZigZagCodec,
};
pub use ext::{
    BinaryReadExt,
    BinaryWriteExt,
    BufReadExt,
    ByteOrder,
    Leb128ReadExt,
    Leb128WriteExt,
    ReadExt,
    ReadSeekExt,
    SeekExt,
    StringReadExt,
    StringWriteExt,
    WriteSeekExt,
    ZigZagReadExt,
    ZigZagWriteExt,
};
pub use stream::{
    BinaryReader,
    BinaryWriter,
    Leb128Reader,
    Leb128Writer,
    ZigZagReader,
    ZigZagWriter,
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
