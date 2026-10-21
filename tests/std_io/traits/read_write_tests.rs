// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::std_io::ReadWrite;

#[test]
fn test_read_write_trait_object_supports_reading_and_writing() {
    let mut cursor = std::io::Cursor::new(b"ping".to_vec());

    {
        let stream: &mut dyn ReadWrite = &mut cursor;
        let mut content = [0_u8; 4];
        stream
            .read_exact(&mut content)
            .expect("read-write trait object should read");
        assert_eq!(*b"ping", content);

        stream
            .write_all(b" pong")
            .expect("read-write trait object should write");
    }

    assert_eq!(b"ping pong", cursor.get_ref().as_slice());
}
