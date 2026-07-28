// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Extension traits for standard I/O.
//!
//! Internal implementation helpers remain crate-private and are not part of the
//! public API.
mod buf_read_ext;
pub(crate) mod internal;
mod read_ext;
mod read_seek_ext;
mod seek_ext;
mod write_ext;
mod write_seek_ext;

pub use buf_read_ext::BufReadExt;
pub use read_ext::ReadExt;
pub use read_seek_ext::ReadSeekExt;
pub use seek_ext::SeekExt;
pub use write_ext::WriteExt;
pub use write_seek_ext::WriteSeekExt;
