// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_io::{
    BoxInput,
    Input,
};

struct CharInput {
    items: Vec<char>,
    position: usize,
}

impl Input for CharInput {
    type Item = char;

    fn is_buffered(&self) -> bool {
        true
    }

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [char],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let remaining = self.items.len() - self.position;
        let read = remaining.min(count);
        output[index..index + read]
            .copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[test]
fn test_box_input_forwards_trait_object_input() {
    let inner: Box<dyn Input<Item = char>> = Box::new(CharInput {
        items: vec!['a', 'b', 'c', 'd', 'e'],
        position: 0,
    });
    let mut input = BoxInput::new(inner);
    let mut output = ['\0'; 2];

    assert!(input.is_buffered());
    assert!(input.get_ref().is_buffered());
    assert!(input.get_mut().is_buffered());
    // SAFETY: `output[1..2]` is a valid destination range.
    assert_eq!(
        1,
        unsafe { input.read_unchecked(&mut output, 1, 1) }
            .expect("unchecked read should succeed")
    );
    assert_eq!(['\0', 'a'], output);
    assert_eq!(
        1,
        input.read(&mut output[..1]).expect("read should succeed")
    );
    assert_eq!(['b', 'a'], output);
    // SAFETY: `output[1..2]` is a valid destination range.
    assert_eq!(
        1,
        unsafe { input.read_fully_unchecked(&mut output, 1, 1) }
            .expect("unchecked complete read should succeed")
    );
    assert_eq!(['b', 'c'], output);
    assert_eq!(
        1,
        input
            .read_fully(&mut output[..1])
            .expect("complete read should succeed")
    );
    assert_eq!('d', output[0]);

    let mut inner = input.into_inner();
    assert_eq!(
        1,
        inner.read(&mut output[..1]).expect("read should succeed")
    );
    assert_eq!('e', output[0]);
}
