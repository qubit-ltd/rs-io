// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow coverage-cfg
mod buf_read_ext;
mod input_ext;
pub mod internal;
mod output_ext;
mod read_ext;
mod read_seek_ext;
mod seek_ext;
mod write_ext;
mod write_seek_ext;

pub use buf_read_ext::BufReadExt;
pub use input_ext::InputExt;
#[cfg(coverage)]
pub use input_ext::{
    coverage_fail_next_add_copied, coverage_natural_add_copied_overflow,
    coverage_reset_add_copied_hooks,
};
pub use output_ext::OutputExt;
pub use read_ext::ReadExt;
pub use read_seek_ext::ReadSeekExt;
pub use seek_ext::SeekExt;
pub use write_ext::WriteExt;
pub use write_seek_ext::WriteSeekExt;
