// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared byte-stream buffering primitives.
//!
//! This module contains byte-oriented buffering types used by higher-level
//! stream adapters. It does not know about binary codecs, text encodings, or
//! record formats; callers compose those concerns on top of the byte windows
//! exposed here.
//!
//! [`BufferedInput`] buffers bytes in front of a [`std::io::Read`] value,
//! implements [`std::io::BufRead`], and exposes the currently unread byte
//! window. [`BufferedOutput`] buffers bytes before a [`std::io::Write`]
//! value and exposes spare writable capacity for hot-path encoders. [`Buffer`]
//! is the low-level position/limit storage object shared by both
//! implementations.
//!
//! The default capacity used by buffered byte input and output is
//! [`DEFAULT_BUFFER_CAPACITY`].

mod buffer;
mod buffered_input;
mod buffered_output;
mod capacity;

pub use buffer::Buffer;
pub use buffered_input::BufferedInput;
pub use buffered_output::BufferedOutput;
pub use capacity::DEFAULT_BUFFER_CAPACITY;
