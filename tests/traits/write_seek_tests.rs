// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::SeekFrom;

use qubit_io::WriteSeek;

#[test]
fn test_write_seek_trait_object_supports_writing_and_seeking() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let writer: &mut dyn WriteSeek = &mut cursor;

    writer
        .write_all(&[0])
        .expect("write-seek trait object should write placeholder");
    writer
        .write_all(b"payload")
        .expect("write-seek trait object should write payload");
    writer
        .seek(SeekFrom::Start(0))
        .expect("write-seek trait object should seek");
    writer
        .write_all(&[7])
        .expect("write-seek trait object should overwrite placeholder");

    assert_eq!(b"\x07payload", cursor.get_ref().as_slice());
}
