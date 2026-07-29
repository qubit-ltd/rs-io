// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Internal helpers that enforce synchronous and asynchronous I/O contracts.

mod async_contract_error;
mod io_contract;

pub(crate) use io_contract::{
    normalize_async_error,
    validate_read_count,
    validate_write_count,
};
