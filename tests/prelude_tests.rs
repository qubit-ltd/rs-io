/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::io::{
    Cursor,
    Seek,
    SeekFrom,
};

use qubit_io::prelude::{
    ReadExt,
    ReadSeek,
    SeekExt,
};

fn takes_read_seek(_stream: &mut dyn ReadSeek) {}

#[test]
fn test_prelude_imports_extension_and_composition_traits() {
    let mut cursor = Cursor::new(b"abcdef".to_vec());
    let mut buffer = [0; 3];

    let count = cursor
        .read_exact_or_eof(&mut buffer)
        .expect("ReadExt should be in prelude");

    assert_eq!(3, count);
    assert_eq!(b"abc", &buffer);

    cursor
        .seek(SeekFrom::Start(1))
        .expect("cursor should seek to test stream size");
    assert_eq!(
        6,
        cursor.stream_size().expect("SeekExt should be in prelude")
    );

    takes_read_seek(&mut cursor);
}
