// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared unit-oriented buffering primitives.
//!
//! This module contains unit-oriented buffering types used by higher-level
//! stream adapters. It does not know about binary codecs, text encodings, or
//! record formats; callers compose those concerns on top of the unit windows
//! exposed here.
//!
//! [`BufferedInput`] buffers units in front of an [`crate::Input`] value,
//! implements [`crate::Input`] and [`crate::Seekable`], and exposes the
//! currently unread unit window. [`BufferedOutput`] buffers units before an
//! [`crate::Output`] value, implements [`crate::Output`] and
//! [`crate::Seekable`], and exposes spare writable capacity for hot-path
//! encoders. [`Buffer`] is the low-level position/limit storage object shared
//! by both implementations.
//!
//! The default capacity used by buffered input and output is
//! [`DEFAULT_BUFFER_CAPACITY`].

mod buffer;
mod buffered_input;
mod buffered_output;
mod capacity;

pub use buffer::Buffer;
pub use buffered_input::BufferedInput;
pub use buffered_output::BufferedOutput;
pub use capacity::DEFAULT_BUFFER_CAPACITY;
