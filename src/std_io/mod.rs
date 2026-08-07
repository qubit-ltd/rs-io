// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integrations with standard-library I/O traits.

pub mod ext;

mod impls;
mod traits;

pub use traits::BufReadSeek;
pub use traits::ReadSeek;
pub use traits::ReadWrite;
pub use traits::ReadWriteSeek;
pub use traits::WriteSeek;
