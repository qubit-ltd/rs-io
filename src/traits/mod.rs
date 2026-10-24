// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Core synchronous, asynchronous, pinned, and seekable I/O traits.

mod async_byte_input;
mod async_byte_output;
mod async_char_input;
mod async_char_output;
mod async_close;
mod async_input;
mod async_output;
mod byte_input;
mod byte_output;
mod char_input;
mod char_output;
mod input;
mod internal;
mod output;
mod pinned_async_input_ext;
mod pinned_async_output_ext;
mod seekable;
mod seekable_byte_input;
mod seekable_byte_output;
mod seekable_char_input;
mod seekable_char_output;
mod seekable_input;
mod seekable_output;

pub use async_byte_input::AsyncByteInput;
pub use async_byte_output::AsyncByteOutput;
pub use async_char_input::AsyncCharInput;
pub use async_char_output::AsyncCharOutput;
pub use async_close::AsyncClose;
pub use async_input::AsyncInput;
pub use async_output::AsyncOutput;
pub use byte_input::ByteInput;
pub use byte_output::ByteOutput;
pub use char_input::CharInput;
pub use char_output::CharOutput;
pub use input::Input;
pub(crate) use internal::normalize_async_error;
pub(crate) use internal::validate_read_count;
pub(crate) use internal::validate_write_count;
pub use output::Output;
pub use pinned_async_input_ext::PinnedAsyncInputExt;
pub use pinned_async_output_ext::PinnedAsyncOutputExt;
pub use seekable::Seekable;
pub use seekable_byte_input::SeekableByteInput;
pub use seekable_byte_output::SeekableByteOutput;
pub use seekable_char_input::SeekableCharInput;
pub use seekable_char_output::SeekableCharOutput;
pub use seekable_input::SeekableInput;
pub use seekable_output::SeekableOutput;
