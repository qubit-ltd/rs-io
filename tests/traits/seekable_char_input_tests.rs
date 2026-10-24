// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::SeekFrom;

use qubit_io::CountingInput;
use qubit_io::SeekableCharInput;
use qubit_io::SeekableInput;

use super::seekable_char_support::CharStream;

fn transfer<S: SeekableCharInput + ?Sized>(stream: &mut S) {
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("seek should succeed"), 1);
    let mut item = ['\0'];
    stream.read_exactly(&mut item).expect("read should succeed");
    assert_eq!(item, ['🦀']);
    assert_eq!(stream.seek_to(SeekFrom::Current(0)).expect("query should succeed"), 2);
}

#[test]
fn test_parameter_and_opaque_return() {
    fn stream() -> impl SeekableCharInput {
        CharStream::new()
    }
    transfer(&mut stream());
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_bound() {
    let mut stream = CharStream::new();
    let stream: &mut dyn SeekableInput<Item = char, Unit = char> = &mut stream;
    transfer(stream);
}

#[test]
fn test_trait_object_preserves_wrapper_seek_capability() {
    let mut stream: Box<dyn SeekableCharInput> = Box::new(CountingInput::new(CharStream::new()));
    transfer(stream.as_mut());
}
