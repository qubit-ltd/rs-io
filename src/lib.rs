// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! # Qubit IO
//!
//! Runtime-neutral synchronous and asynchronous item-stream I/O for Rust.
//!
//! [`Input`] and [`Output`] model synchronous item transfer. [`AsyncInput`] and
//! [`AsyncOutput`] expose the same boundary through `Pin` and `Poll` without
//! selecting an executor. Standard-library byte streams implement the
//! synchronous traits automatically; optional newtypes bridge Tokio and
//! `futures-io` asynchronous streams without overlapping blanket impls.
//! [`AsyncClose`] represents a real close operation rather than a flush.
//! Named futures preserve multi-poll progress, while [`PinnedAsyncInputExt`]
//! and [`PinnedAsyncOutputExt`] support pinned `!Unpin` values and trait
//! objects. Async implementations must register the current waker before
//! returning `Pending`, transfer no items on `Pending` or errors, and prevent
//! `WouldBlock` and `Interrupted` from crossing the trait boundary.
//!
//! Item-oriented buffering includes [`BufferedInput`], [`BufferedOutput`],
//! [`AsyncBufferedInput`], and [`AsyncBufferedOutput`]. Limit, counting, and
//! checksum wrappers are available for both operation modes. Binary and text
//! codecs remain in sibling crates.
//!
//! The concrete trait definitions and wrapper types live in dedicated modules
//! and are re-exported from the crate root for ergonomic use.
// qubit-style: allow coverage-cfg

mod adapters;
mod async_io;
pub mod buffered;
mod capacity_const;
pub mod ext;
mod into_inner_error;
mod traits;
mod util;
mod wrappers;

pub use adapters::{
    BoxInput,
    BoxOutput,
    InputRef,
    OutputRef,
};
#[cfg(feature = "futures-io")]
pub use adapters::{
    FuturesAsyncRead,
    FuturesAsyncWrite,
    FuturesInput,
    FuturesOutput,
};
#[cfg(feature = "tokio")]
pub use adapters::{
    TokioAsyncRead,
    TokioAsyncWrite,
    TokioInput,
    TokioOutput,
};
pub use async_io::{
    CloseFuture,
    FlushFuture,
    ReadExactFuture,
    ReadFullyFuture,
    ReadFuture,
    WriteFullyFuture,
    WriteFuture,
};
pub use buffered::{
    AsyncBufferedInput,
    AsyncBufferedOutput,
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
pub use into_inner_error::IntoInnerError;
pub use traits::{
    AsyncClose,
    AsyncInput,
    AsyncOutput,
    BufReadSeek,
    Input,
    Output,
    PinnedAsyncInputExt,
    PinnedAsyncOutputExt,
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
    coverage_fail_reserve_above,
    coverage_fail_reserve_after,
    coverage_reset_add_item_count_hooks,
    coverage_reset_reserve_hooks,
};
pub use util::{
    try_reserve_string,
    try_reserve_vec,
};
pub use wrappers::{
    AsyncChecksumInput,
    AsyncChecksumOutput,
    AsyncCountingInput,
    AsyncCountingOutput,
    AsyncLimitInput,
    AsyncLimitOutput,
    ChecksumInput,
    ChecksumOutput,
    CountingInput,
    CountingOutput,
    LimitInput,
    LimitOutput,
    PositionGuard,
    SyncSeekTeeInput,
    TeeInput,
    TeeOutput,
};
