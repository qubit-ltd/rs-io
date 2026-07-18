// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::FuturesOutput;
use std::io::Cursor;

#[test]
fn test_futures_output_type_is_public() {
    assert!(
        std::any::type_name::<FuturesOutput<Cursor<Vec<u8>>>>()
            .contains("FuturesOutput")
    );
}
