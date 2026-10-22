// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_io::CharInput;
use qubit_io::Input;

use super::specialized_stream_support::ItemInput;

fn read<I: CharInput + ?Sized>(input: &mut I) -> [char; 2] {
    let mut chars = ['\0'; 2];
    input
        .read_exactly(&mut chars)
        .expect("input should supply two Unicode scalar values");
    chars
}

#[test]
fn test_parameter_and_opaque_return_read_unicode_scalars() {
    fn input() -> impl CharInput {
        ItemInput::new(&['你', '🦀'])
    }
    assert_eq!(read(&mut input()), ['你', '🦀']);
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_char_bound() {
    let mut input = ItemInput::new(&['你', '🦀']);
    let input: &mut dyn Input<Item = char> = &mut input;
    assert_eq!(read(input), ['你', '🦀']);
}

#[test]
fn test_char_trait_object_supports_reads() {
    let mut input: Box<dyn CharInput> = Box::new(ItemInput::new(&['你', '🦀']));
    assert_eq!(read(input.as_mut()), ['你', '🦀']);
}
