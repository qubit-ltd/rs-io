/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::SeekFrom;

use qubit_io::ReadWriteSeek;

#[test]
fn test_read_write_seek_trait_object_supports_all_io_operations() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let stream: &mut dyn ReadWriteSeek = &mut cursor;

    stream
        .write_all(b"abc")
        .expect("read-write-seek trait object should write");
    stream
        .seek(SeekFrom::Start(0))
        .expect("read-write-seek trait object should seek");
    let mut content = String::new();
    stream
        .read_to_string(&mut content)
        .expect("read-write-seek trait object should read");

    assert_eq!("abc", content);
}
