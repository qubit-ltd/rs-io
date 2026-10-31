// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_io::CharOutput;
use qubit_io::Output;

struct ItemOutput<'a, T> {
    items: &'a mut Vec<T>,
}

impl<'a, T> ItemOutput<'a, T> {
    fn new(items: &'a mut Vec<T>) -> Self {
        Self { items }
    }
}

impl<T: Copy> Output for ItemOutput<'_, T> {
    type Item = T;

    unsafe fn write_unchecked(&mut self, input: &[T], index: usize, count: usize) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write<O: CharOutput + ?Sized>(output: &mut O) {
    output
        .write_fully(&['你', '🦀'])
        .expect("output should accept two Unicode scalar values");
    output.flush().expect("character output should flush");
}

#[test]
fn test_parameter_and_borrowed_opaque_return_write_unicode_scalars() {
    fn output(chars: &mut Vec<char>) -> impl CharOutput + '_ {
        ItemOutput::new(chars)
    }
    let mut chars = Vec::new();
    write(&mut output(&mut chars));
    assert_eq!(chars, ['你', '🦀']);
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_char_bound() {
    let mut chars = Vec::new();
    {
        let mut output = ItemOutput::new(&mut chars);
        let output: &mut dyn Output<Item = char> = &mut output;
        write(output);
    }
    assert_eq!(chars, ['你', '🦀']);
}

#[test]
fn test_char_trait_object_supports_writes() {
    let mut chars = Vec::new();
    {
        let mut output: Box<dyn CharOutput + '_> = Box::new(ItemOutput::new(&mut chars));
        write(output.as_mut());
    }
    assert_eq!(chars, ['你', '🦀']);
}
