// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::ReadWrite;

#[test]
fn test_read_write_trait_object_supports_reading_and_writing() {
    let mut cursor = std::io::Cursor::new(Vec::new());

    {
        let stream: &mut dyn ReadWrite = &mut cursor;
        stream
            .write_all(b"ping")
            .expect("read-write trait object should write");
    }

    assert_eq!(b"ping", cursor.get_ref().as_slice());
}
