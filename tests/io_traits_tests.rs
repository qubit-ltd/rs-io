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

use qubit_io::{ReadSeek, ReadWrite, ReadWriteSeek, WriteSeek};

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

#[test]
fn test_read_write_and_write_seek_trait_objects_compile() {
    let mut read_write_cursor = std::io::Cursor::new(Vec::new());
    let read_write: &mut dyn ReadWrite = &mut read_write_cursor;
    read_write
        .write_all(b"x")
        .expect("read-write trait object should write");

    let mut write_seek_cursor = std::io::Cursor::new(Vec::new());
    let write_seek: &mut dyn WriteSeek = &mut write_seek_cursor;
    write_seek
        .write_all(b"y")
        .expect("write-seek trait object should write");
    write_seek
        .seek(SeekFrom::Start(0))
        .expect("write-seek trait object should seek");
}
