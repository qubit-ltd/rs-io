// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Futures-io output adapter.

#[path = "futures_output/futures_output.rs"]
mod implementation;

pub use implementation::FuturesOutput;
