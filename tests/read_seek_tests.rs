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

use qubit_io::ReadSeek;

#[test]
fn test_read_seek_trait_object_supports_reading_and_seeking() {
    let mut cursor = std::io::Cursor::new(b"abcdef".to_vec());
    let reader: &mut dyn ReadSeek = &mut cursor;

    let mut prefix = [0; 3];
    reader
        .read_exact(&mut prefix)
        .expect("read-seek trait object should read");
    reader
        .seek(SeekFrom::Start(1))
        .expect("read-seek trait object should seek");
    let mut byte = [0; 1];
    reader
        .read_exact(&mut byte)
        .expect("read-seek trait object should read after seek");

    assert_eq!(b"abc", &prefix);
    assert_eq!(b"b", &byte);
}
