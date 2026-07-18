// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::TokioOutput;
use std::io::Cursor;

#[test]
fn test_tokio_output_type_is_public() {
    assert!(
        std::any::type_name::<TokioOutput<Cursor<Vec<u8>>>>()
            .contains("TokioOutput")
    );
}
