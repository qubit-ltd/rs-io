// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_io::BoxOutput;
use qubit_io::Output;

struct CharOutput {
    items: Vec<char>,
    flushed: bool,
}

impl Output for CharOutput {
    type Item = char;

    fn is_buffered(&self) -> bool {
        true
    }

    unsafe fn write_unchecked(&mut self, input: &[char], index: usize, count: usize) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushed = true;
        Ok(())
    }
}

#[test]
fn test_box_output_forwards_trait_object_output() {
    let inner: Box<dyn Output<Item = char>> = Box::new(CharOutput {
        items: Vec::new(),
        flushed: false,
    });
    let mut output = BoxOutput::new(inner);

    assert!(output.is_buffered());
    assert!(output.get_ref().is_buffered());
    assert!(output.get_mut().is_buffered());
    // SAFETY: `['a', 'b'][1..2]` is a valid source range.
    assert_eq!(
        1,
        unsafe { output.write_unchecked(&['a', 'b'], 1, 1) }.expect("unchecked write should succeed")
    );
    assert_eq!(1, output.write(&['c']).expect("write should succeed"));
    // SAFETY: `['d', 'e'][0..1]` is a valid source range.
    unsafe { output.write_fully_unchecked(&['d', 'e'], 0, 1) }.expect("unchecked complete write should succeed");
    output.write_fully(&['e']).expect("complete write should succeed");
    output.flush().expect("flush should succeed");

    let inner = output.into_inner();
    assert!(inner.is_buffered());
}
