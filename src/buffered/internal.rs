// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private implementation stages shared by buffered input modules.

#[path = "internal/buffered_input_read.rs"]
mod buffered_input_read;

pub(super) use buffered_input_read::ensure_available_impl;
pub(super) use buffered_input_read::fill_more_impl;
pub(super) use buffered_input_read::fill_until_impl;
pub(super) use buffered_input_read::read_fully_unchecked_impl;
pub(super) use buffered_input_read::read_unchecked_impl;
