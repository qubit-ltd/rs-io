// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::SyncSeekTeeReader;
use std::io::Cursor;

#[test]
fn test_sync_seek_tee_reader_exposes_both_streams() {
    let reader = Cursor::new(b"abc".to_vec());
    let branch = Cursor::new(Vec::<u8>::new());
    let tee = SyncSeekTeeReader::new(reader, branch);
    assert_eq!(0, tee.inner().position());
    assert_eq!(0, tee.branch().position());
}
