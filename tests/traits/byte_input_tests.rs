// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::Read;

use qubit_io::ByteInput;
use qubit_io::CountingInput;
use qubit_io::Input;
use qubit_io::InputRef;

#[test]
fn test_byte_input_parameter_and_opaque_return() {
    fn create_input() -> impl ByteInput {
        Cursor::new(b"bytes")
    }

    fn read_bytes<I: ByteInput>(mut input: I) -> [u8; 5] {
        let mut bytes = [0; 5];
        input
            .read_exactly(&mut bytes)
            .expect("byte input should read all bytes");
        bytes
    }

    assert_eq!(read_bytes(create_input()), *b"bytes");
}

#[test]
fn test_item_wrapper_automatically_implements_byte_input() {
    fn read_byte(input: &mut impl ByteInput) -> u8 {
        let mut byte = [0];
        input.read_exactly(&mut byte).expect("wrapper should supply a byte");
        byte[0]
    }

    let mut input = CountingInput::new(Cursor::new(b"x"));
    assert_eq!(read_byte(&mut input), b'x');
    assert_eq!(input.bytes_read(), 1);
}

#[test]
fn test_unsized_input_automatically_implements_byte_input() {
    fn read_byte<I: ByteInput + ?Sized>(input: &mut I) -> u8 {
        let mut byte = [0];
        input
            .read_exactly(&mut byte)
            .expect("trait object should supply a byte");
        byte[0]
    }

    let mut cursor = Cursor::new(b"xy");
    let input: &mut dyn Input<Item = u8> = &mut cursor;
    assert_eq!(read_byte(input), b'x');
    let input: &mut dyn Read = &mut cursor;
    assert_eq!(read_byte(input), b'y');
}

#[test]
fn test_byte_input_trait_object_converts_to_std_read() {
    let mut input: Box<dyn ByteInput> = Box::new(CountingInput::new(Cursor::new(b"bytes")));
    let mut reader = InputRef::new(input.as_mut()).into_std_read();
    let mut bytes = Vec::new();
    Read::read_to_end(&mut reader, &mut bytes).expect("standard adapter should read to EOF");
    assert_eq!(bytes, b"bytes");
}
