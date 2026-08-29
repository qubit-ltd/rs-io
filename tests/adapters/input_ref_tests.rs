// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_io::Input;
use qubit_io::InputRef;

struct CharInput {
    items: Vec<char>,
    position: usize,
    buffered: bool,
}

impl CharInput {
    fn new(items: Vec<char>, buffered: bool) -> Self {
        Self {
            items,
            position: 0,
            buffered,
        }
    }
}

impl Input for CharInput {
    type Item = char;

    fn is_buffered(&self) -> bool {
        self.buffered
    }

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> io::Result<usize> {
        let remaining = self.items.len() - self.position;
        let read = remaining.min(count);
        output[index..index + read].copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[test]
fn test_input_ref_forwards_reads_and_exposes_borrowed_input() {
    let mut inner = CharInput::new(vec!['a', 'b', 'c', 'd', 'e'], true);
    let mut input = InputRef::new(&mut inner);
    let mut output = ['\0'; 2];

    assert!(input.is_buffered());
    assert!(input.get_ref().buffered);
    input.get_mut().buffered = false;
    assert!(!input.get_ref().buffered);

    // SAFETY: `output[1..2]` is a valid destination range.
    assert_eq!(
        1,
        unsafe { input.read_unchecked(&mut output, 1, 1) }.expect("unchecked read should succeed")
    );
    assert_eq!(['\0', 'a'], output);
    assert_eq!(1, input.read(&mut output[..1]).expect("read should succeed"));
    assert_eq!(['b', 'a'], output);

    // SAFETY: `output[1..2]` is a valid destination range.
    assert_eq!(
        1,
        unsafe { input.read_fully_unchecked(&mut output, 1, 1) }.expect("unchecked complete read should succeed")
    );
    assert_eq!(['b', 'c'], output);
    assert_eq!(
        1,
        input
            .read_fully(&mut output[..1])
            .expect("complete read should succeed")
    );
    assert_eq!('d', output[0]);

    let inner = input.into_inner();
    let mut output = ['\0'; 1];
    assert_eq!(1, inner.read(&mut output).expect("read should succeed"));
    assert_eq!(['e'], output);
}
