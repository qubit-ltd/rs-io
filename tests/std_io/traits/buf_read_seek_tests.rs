// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{
    BufReader,
    Cursor,
    SeekFrom,
};

use qubit_io::std_io::BufReadSeek;

#[test]
fn test_buf_read_seek_trait_object_supports_buffered_reading_and_seeking() {
    let cursor = Cursor::new(b"abc\ndef".to_vec());
    let mut reader = BufReader::new(cursor);
    let stream: &mut dyn BufReadSeek = &mut reader;

    let buffered = stream
        .fill_buf()
        .expect("buf-read-seek trait object should fill buffer");
    assert!(buffered.starts_with(b"abc"));
    stream
        .seek(SeekFrom::Start(4))
        .expect("buf-read-seek trait object should seek");

    let mut line = String::new();
    stream
        .read_line(&mut line)
        .expect("buf-read-seek trait object should read line after seek");
    assert_eq!("def", line);
}
