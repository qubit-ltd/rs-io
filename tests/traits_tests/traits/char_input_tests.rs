// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::VecDeque;
use std::io;

use qubit_io::CharInput;
use qubit_io::Input;

struct ItemInput<T> {
    items: VecDeque<T>,
}

impl<T: Copy> ItemInput<T> {
    fn new(items: &[T]) -> Self {
        Self {
            items: items.iter().copied().collect(),
        }
    }
}

impl<T: Copy> Input for ItemInput<T> {
    type Item = T;

    unsafe fn read_unchecked(&mut self, output: &mut [T], index: usize, count: usize) -> io::Result<usize> {
        let read = count.min(self.items.len());
        for target in &mut output[index..index + read] {
            *target = self.items.pop_front().expect("read count must fit the input");
        }
        Ok(read)
    }
}

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
