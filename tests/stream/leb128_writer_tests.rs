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
    Write,
};

use qubit_io::{
    Leb128ReadExt,
    Leb128Writer,
    StringReadExt,
};

#[test]
fn test_leb128_writer_writes_boundary_values_and_strings() {
    let mut writer = Leb128Writer::new(Vec::new());

    writer.write_u8(0).expect("u8 zero should be written");
    writer.write_u8(u8::MAX).expect("u8 max should be written");
    writer
        .write_u16(u16::MAX)
        .expect("u16 max should be written");
    writer
        .write_u32(u32::MAX)
        .expect("u32 max should be written");
    writer
        .write_u64(u64::MAX)
        .expect("u64 max should be written");
    writer
        .write_u128(u128::MAX)
        .expect("u128 max should be written");
    writer
        .write_usize(usize::MAX)
        .expect("usize max should be written");
    writer.write_i8(0).expect("i8 zero should be written");
    writer.write_i8(i8::MIN).expect("i8 min should be written");
    writer
        .write_i16(i16::MAX)
        .expect("i16 max should be written");
    writer
        .write_i32(i32::MIN)
        .expect("i32 min should be written");
    writer
        .write_i64(i64::MAX)
        .expect("i64 max should be written");
    writer
        .write_i128(i128::MIN)
        .expect("i128 min should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize min should be written");
    writer
        .write_utf8_string("hello 世界")
        .expect("string should be written");
    writer.get_mut().push(0x7f);

    assert_eq!(
        0x7f,
        *writer.get_ref().last().expect("tail byte should exist")
    );

    let mut reader = Cursor::new(writer.into_inner());
    assert_eq!(0, reader.read_uleb_u8().expect("u8 zero should be read"));
    assert_eq!(
        u8::MAX,
        reader.read_uleb_u8().expect("u8 max should be read")
    );
    assert_eq!(
        u16::MAX,
        reader.read_uleb_u16().expect("u16 max should be read")
    );
    assert_eq!(
        u32::MAX,
        reader.read_uleb_u32().expect("u32 max should be read")
    );
    assert_eq!(
        u64::MAX,
        reader.read_uleb_u64().expect("u64 max should be read")
    );
    assert_eq!(
        u128::MAX,
        reader.read_uleb_u128().expect("u128 max should be read")
    );
    assert_eq!(
        usize::MAX,
        reader.read_uleb_usize().expect("usize max should be read")
    );
    assert_eq!(0, reader.read_sleb_i8().expect("i8 zero should be read"));
    assert_eq!(
        i8::MIN,
        reader.read_sleb_i8().expect("i8 min should be read")
    );
    assert_eq!(
        i16::MAX,
        reader.read_sleb_i16().expect("i16 max should be read")
    );
    assert_eq!(
        i32::MIN,
        reader.read_sleb_i32().expect("i32 min should be read")
    );
    assert_eq!(
        i64::MAX,
        reader.read_sleb_i64().expect("i64 max should be read")
    );
    assert_eq!(
        i128::MIN,
        reader.read_sleb_i128().expect("i128 min should be read")
    );
    assert_eq!(
        isize::MIN,
        reader.read_sleb_isize().expect("isize min should be read")
    );
    assert_eq!(
        "hello 世界",
        reader
            .read_utf8_string_uleb(32)
            .expect("string should be read")
    );
    assert_eq!(0x7f, reader.get_ref()[reader.position() as usize]);
}

#[test]
fn test_leb128_writer_delegates_raw_write_and_flush() {
    let mut writer = Leb128Writer::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer
        .write_all(&[0x01, 0x02])
        .expect("raw bytes should be written");
    writer
        .flush()
        .expect("flush should delegate to inner writer");

    assert_eq!(vec![0x01, 0x02], writer.into_inner());
}

#[test]
fn test_leb128_writer_forwards_seek() {
    use std::io::{
        Seek,
        SeekFrom,
    };

    let mut writer = Leb128Writer::new(Cursor::new(Vec::new()));

    writer
        .write_all(b"abc")
        .expect("initial write should succeed");
    writer
        .seek(SeekFrom::Start(1))
        .expect("seek should be forwarded");
    writer.write_all(b"z").expect("patch write should succeed");

    assert_eq!(b"azc", writer.into_inner().into_inner().as_slice());
}
