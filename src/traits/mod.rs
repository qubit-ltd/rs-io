// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Core synchronous, asynchronous, pinned, and seekable I/O traits.

mod async_close;
mod async_input;
mod async_output;
mod input;
mod internal;
mod output;
mod pinned_async_input_ext;
mod pinned_async_output_ext;
mod seekable;
mod seekable_input;
mod seekable_output;

pub use async_close::AsyncClose;
pub use async_input::AsyncInput;
pub use async_output::AsyncOutput;
pub use input::Input;
pub(crate) use internal::{
    normalize_async_error,
    validate_read_count,
    validate_write_count,
};
pub use output::Output;
pub use pinned_async_input_ext::PinnedAsyncInputExt;
pub use pinned_async_output_ext::PinnedAsyncOutputExt;
pub use seekable::Seekable;
pub use seekable_input::SeekableInput;
pub use seekable_output::SeekableOutput;
