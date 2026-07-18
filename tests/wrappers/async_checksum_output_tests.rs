// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::support_tests::TestStream;
use qubit_io::AsyncChecksumOutput;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

#[test]
fn test_async_checksum_output_starts_with_hasher_checksum() {
    let hasher = DefaultHasher::new();
    let expected = hasher.finish();
    let output = AsyncChecksumOutput::new(TestStream, hasher);
    assert_eq!(expected, output.checksum());
}
