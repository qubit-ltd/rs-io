// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::SeekFrom;

use qubit_io::CountingOutput;
use qubit_io::SeekableCharOutput;
use qubit_io::SeekableOutput;

use super::seekable_char_support::CharStream;

fn transfer<S: SeekableCharOutput + ?Sized>(stream: &mut S) {
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("seek should succeed"), 1);
    stream.write_fully(&['好']).expect("write should succeed");
    assert_eq!(stream.seek_to(SeekFrom::Current(0)).expect("query should succeed"), 2);
}

#[test]
fn test_parameter_and_opaque_return() {
    fn stream() -> impl SeekableCharOutput {
        CharStream::new()
    }
    transfer(&mut stream());
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_bound() {
    let mut stream = CharStream::new();
    let stream: &mut dyn SeekableOutput<Item = char, Unit = char> = &mut stream;
    transfer(stream);
}

#[test]
fn test_trait_object_preserves_wrapper_seek_capability() {
    let mut stream: Box<dyn SeekableCharOutput> = Box::new(CountingOutput::new(CharStream::new()));
    transfer(stream.as_mut());
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("rewind should succeed"), 1);
    stream.write_fully(&['好']).expect("overwrite should succeed");
}

#[test]
fn test_seek_overwrites_one_char() {
    let mut stream = CharStream::new();
    transfer(&mut stream);
    assert_eq!(stream.items, ['你', '好', '\u{0301}']);
}
