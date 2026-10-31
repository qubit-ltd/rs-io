// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::SeekFrom;

use qubit_io::CountingOutput;
use qubit_io::SeekableByteOutput;
use qubit_io::SeekableOutput;

fn transfer<S: SeekableByteOutput + ?Sized>(stream: &mut S) {
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("seek should succeed"), 1);
    stream.write_fully(b"x").expect("write should succeed");
    assert_eq!(stream.seek_to(SeekFrom::Current(0)).expect("query should succeed"), 2);
}

#[test]
fn test_parameter_and_opaque_return() {
    fn stream() -> impl SeekableByteOutput {
        Cursor::new(b"abc".to_vec())
    }
    transfer(&mut stream());
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_bound() {
    let mut stream = Cursor::new(b"abc".to_vec());
    let stream: &mut dyn SeekableOutput<Item = u8, Unit = u8> = &mut stream;
    transfer(stream);
}

#[test]
fn test_trait_object_preserves_wrapper_seek_capability() {
    let mut stream: Box<dyn SeekableByteOutput> = Box::new(CountingOutput::new(Cursor::new(b"abc".to_vec())));
    transfer(stream.as_mut());
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("rewind should succeed"), 1);
    stream.write_fully(b"x").expect("overwrite should succeed");
}

#[test]
fn test_seek_overwrites_one_byte() {
    let mut stream = Cursor::new(b"abc".to_vec());
    transfer(&mut stream);
    assert_eq!(stream.into_inner(), b"axc");
}
