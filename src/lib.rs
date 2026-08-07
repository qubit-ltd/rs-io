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
pub mod std_io;
mod traits;
mod util;
mod wrappers;

pub use adapters::BoxAsyncInput;
pub use adapters::BoxAsyncOutput;
pub use adapters::BoxInput;
pub use adapters::BoxOutput;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use adapters::FuturesAsyncRead;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use adapters::FuturesAsyncWrite;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use adapters::FuturesInput;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use adapters::FuturesOutput;
pub use adapters::InputRef;
pub use adapters::OutputRef;
#[cfg(all(feature = "tokio", not(miri)))]
pub use adapters::TokioAsyncRead;
#[cfg(all(feature = "tokio", not(miri)))]
pub use adapters::TokioAsyncWrite;
#[cfg(all(feature = "tokio", not(miri)))]
pub use adapters::TokioInput;
#[cfg(all(feature = "tokio", not(miri)))]
pub use adapters::TokioOutput;
pub use async_io::CloseFuture;
pub use async_io::FlushFuture;
pub use async_io::ReadExactFuture;
pub use async_io::ReadFullyFuture;
pub use async_io::ReadFuture;
pub use async_io::WriteFullyFuture;
pub use async_io::WriteFuture;
pub use buffered::AsyncBufferedInput;
pub use buffered::AsyncBufferedOutput;
pub use buffered::Buffer;
pub use buffered::BufferedInput;
pub use buffered::BufferedOutput;
pub use buffered::EnsuredBufferedInput;
pub use buffered::EnsuredBufferedOutput;
pub use capacity_const::DEFAULT_BUFFER_CAPACITY;
pub use capacity_const::DEFAULT_COMPARE_BUFFER_SIZE;
pub use capacity_const::DEFAULT_COPY_BUFFER_SIZE;
pub use traits::AsyncClose;
pub use traits::AsyncInput;
pub use traits::AsyncOutput;
pub use traits::Input;
pub use traits::Output;
pub use traits::PinnedAsyncInputExt;
pub use traits::PinnedAsyncOutputExt;
pub use traits::Seekable;
pub use traits::SeekableInput;
pub use traits::SeekableOutput;
pub use util::Streams;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_add_item_count_overflow;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_fail_next_add_item_count;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_fail_next_reserve;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_fail_next_string_reserve;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_fail_reserve_above;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_fail_reserve_after;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_reset_add_item_count_hooks;
#[cfg(coverage)]
#[doc(hidden)]
pub use util::coverage_reset_reserve_hooks;
pub use wrappers::AsyncChecksumInput;
pub use wrappers::AsyncChecksumOutput;
pub use wrappers::AsyncCountingInput;
pub use wrappers::AsyncCountingOutput;
pub use wrappers::AsyncLimitInput;
pub use wrappers::AsyncLimitOutput;
pub use wrappers::ChecksumInput;
pub use wrappers::ChecksumOutput;
pub use wrappers::CountingInput;
pub use wrappers::CountingOutput;
pub use wrappers::LimitInput;
pub use wrappers::LimitOutput;
pub use wrappers::PositionGuard;
pub use wrappers::SyncSeekTeeInput;
pub use wrappers::TeeInput;
pub use wrappers::TeeOutput;
