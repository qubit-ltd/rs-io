// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared byte-stream buffering primitives.

mod buffer;
mod buffered_byte_input;
mod buffered_byte_output;

pub use buffer::Buffer;
pub use buffered_byte_input::BufferedByteInput;
pub use buffered_byte_output::BufferedByteOutput;
