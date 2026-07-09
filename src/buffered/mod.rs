// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared item-oriented buffering primitives.
//!
//! This module contains item-oriented buffering types used by higher-level
//! stream adapters. It does not know about binary codecs, text encodings, or
//! record formats; callers compose those concerns on top of the item windows
//! exposed here.
//!
//! [`BufferedInput`] buffers items in front of an [`crate::Input`] value,
//! implements [`crate::Input`] and [`crate::Seekable`], and exposes the
//! currently unread item window. [`BufferedOutput`] buffers items before an
//! [`crate::Output`] value, implements [`crate::Output`] and
//! [`crate::Seekable`], and exposes spare writable capacity for hot-path
//! encoders. [`Buffer`] is the low-level position/limit storage object shared
//! by both implementations.
//!
//! The default capacity used by buffered input and output is
//! [`DEFAULT_BUFFER_CAPACITY`].

mod boxed_dyn_input;
mod boxed_dyn_output;
mod buffer;
mod buffered_input;
mod buffered_output;
mod ensured_buffered_input;
mod ensured_buffered_output;

pub use crate::capacity_const::DEFAULT_BUFFER_CAPACITY;
pub(crate) use boxed_dyn_input::BoxedDynInput;
pub(crate) use boxed_dyn_output::BoxedDynOutput;
pub use buffer::Buffer;
pub use buffered_input::BufferedInput;
pub use buffered_output::BufferedOutput;
pub use ensured_buffered_input::EnsuredBufferedInput;
pub use ensured_buffered_output::EnsuredBufferedOutput;
