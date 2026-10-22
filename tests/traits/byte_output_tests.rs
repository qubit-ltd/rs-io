// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Write;

use qubit_io::ByteOutput;
use qubit_io::CountingOutput;
use qubit_io::Output;
use qubit_io::OutputRef;

#[test]
fn test_byte_output_parameter_and_opaque_return() {
    fn create_output(bytes: &mut Vec<u8>) -> impl ByteOutput + '_ {
        bytes
    }

    fn write_bytes<O: ByteOutput>(mut output: O) {
        output
            .write_fully(b"bytes")
            .expect("byte output should write all bytes");
    }

    let mut bytes = Vec::new();
    write_bytes(create_output(&mut bytes));
    assert_eq!(bytes, b"bytes");
}

#[test]
fn test_item_wrapper_automatically_implements_byte_output() {
    fn write_byte(output: &mut impl ByteOutput) {
        output.write_fully(b"x").expect("wrapper should accept a byte");
    }

    let mut bytes = Vec::new();
    let mut output = CountingOutput::new(&mut bytes);
    write_byte(&mut output);
    assert_eq!(output.bytes_written(), 1);
    assert_eq!(bytes, b"x");
}

#[test]
fn test_unsized_output_automatically_implements_byte_output() {
    fn write_byte<O: ByteOutput + ?Sized>(output: &mut O) {
        output.write_fully(b"x").expect("trait object should accept a byte");
    }

    let mut bytes = Vec::new();
    let output: &mut dyn Output<Item = u8> = &mut bytes;
    write_byte(output);
    let output: &mut dyn Write = &mut bytes;
    write_byte(output);
    assert_eq!(bytes, b"xx");
}

#[test]
fn test_byte_output_trait_object_converts_to_std_write() {
    let mut bytes = Vec::new();
    let mut output: Box<dyn ByteOutput + '_> = Box::new(CountingOutput::new(&mut bytes));
    let mut writer = OutputRef::new(output.as_mut()).into_std_write();
    Write::write_all(&mut writer, b"bytes").expect("standard adapter should write all bytes");
    Write::flush(&mut writer).expect("standard adapter should flush");
    drop(output);
    assert_eq!(bytes, b"bytes");
}
