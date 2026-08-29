// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_io::FuturesAsyncWrite;

#[test]
fn test_futures_async_write_type_is_public() {
    assert!(std::any::type_name::<FuturesAsyncWrite<Cursor<Vec<u8>>>>().contains("FuturesAsyncWrite"));
}
