// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::SeekFrom;

use qubit_io::SeekableInput;
use qubit_io::std_io::ReadSeek;

fn read_and_seek<I>(input: &mut I) -> std::io::Result<u8>
where
    I: SeekableInput<Item = u8, Unit = u8> + ?Sized,
{
    let mut byte = [0];
    input.read_exactly(&mut byte)?;
    input.seek_to(SeekFrom::Start(0))?;
    Ok(byte[0])
}

#[test]
fn test_cursor_implements_seekable_input() {
    fn assert_seekable_input<T: SeekableInput>() {}
    assert_seekable_input::<Cursor<Vec<u8>>>();
}

#[test]
fn test_dyn_read_seek_implements_seekable_input() {
    let mut cursor = Cursor::new(vec![42_u8, 7]);
    let input: &mut dyn ReadSeek = &mut cursor;

    assert_eq!(42, read_and_seek(input).expect("read and seek should succeed"));
    assert_eq!(0, input.stream_position().expect("position should be readable"));
}
