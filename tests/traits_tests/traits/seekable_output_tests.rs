// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::SeekFrom;

use qubit_io::SeekableOutput;
use qubit_io::std_io::WriteSeek;

fn write_and_seek<O>(output: &mut O) -> std::io::Result<()>
where
    O: SeekableOutput<Item = u8, Unit = u8> + ?Sized,
{
    output.write_fully(&[42_u8, 7])?;
    output.seek_to(SeekFrom::Start(0))?;
    Ok(())
}

#[test]
fn test_cursor_implements_seekable_output() {
    fn assert_seekable_output<T: SeekableOutput>() {}
    assert_seekable_output::<Cursor<Vec<u8>>>();
}

#[test]
fn test_dyn_write_seek_implements_seekable_output() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let output: &mut dyn WriteSeek = &mut cursor;

        write_and_seek(output).expect("write and seek should succeed");
        assert_eq!(0, output.stream_position().expect("position should be readable"));
    }
    assert_eq!(0, cursor.position());
    assert_eq!(&[42_u8, 7], cursor.get_ref().as_slice());
}
