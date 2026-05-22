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
    BigEndian,
    ByteOrder,
    ByteOrderSpec,
    Leb128ReadExt,
    Leb128WriteExt,
    ReadExt,
    ReadSeek,
    SeekExt,
    ZigZagReadExt,
    ZigZagWriteExt,
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

#[test]
fn test_prelude_imports_codec_markers_and_encoding_extension_traits() {
    assert_eq!(ByteOrder::BigEndian, BigEndian::ORDER);

    let mut buffer = Vec::new();
    buffer
        .write_uleb_u16(300)
        .expect("Leb128WriteExt should be in prelude");
    buffer
        .write_zig_zag_i16(-42)
        .expect("ZigZagWriteExt should be in prelude");

    let mut input = Cursor::new(buffer);
    assert_eq!(
        300,
        input
            .read_uleb_u16()
            .expect("Leb128ReadExt should be in prelude")
    );
    assert_eq!(
        -42,
        input
            .read_zig_zag_i16()
            .expect("ZigZagReadExt should be in prelude")
    );
}
