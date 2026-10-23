// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use qubit_io::Input;
use qubit_io::InputRef;

struct VectorCharInput;
impl Input for VectorCharInput {
    type Item = char;
    unsafe fn read_unchecked(&mut self, _: &mut [char], _: usize, _: usize) -> std::io::Result<usize> {
        panic!("scalar read called")
    }
    fn read_vectored(&mut self, buffers: &mut [&mut [char]]) -> std::io::Result<usize> {
        let mut chars = ['甲', '乙', '丙', '丁'].into_iter();
        let mut total = 0;
        for buffer in buffers {
            for slot in buffer.iter_mut() {
                let Some(value) = chars.next() else {
                    return Ok(total);
                };
                *slot = value;
                total += 1;
            }
        }
        Ok(total)
    }
}

#[test]
fn vectored_forward_input_ref_keeps_override() {
    let mut source = VectorCharInput;
    let borrowed: &mut dyn Input<Item = char> = &mut source;
    let mut input = InputRef::new(borrowed);
    let mut first = ['!'; 2];
    let mut second = ['?'; 2];
    assert_eq!(
        Input::read_vectored(&mut input, &mut [&mut first, &mut second]).unwrap(),
        4
    );
    assert_eq!((&first, &second), (&['甲', '乙'], &['丙', '丁']));
}

struct EmptyErrorInput(Rc<Cell<usize>>);

impl Input for EmptyErrorInput {
    type Item = char;

    unsafe fn read_unchecked(&mut self, _: &mut [char], _: usize, _: usize) -> std::io::Result<usize> {
        panic!("scalar read called")
    }

    fn read_vectored(&mut self, _: &mut [&mut [char]]) -> std::io::Result<usize> {
        self.0.set(self.0.get() + 1);
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    }
}

#[test]
fn vectored_forward_input_ref_preserves_empty_list_error() {
    let calls = Rc::new(Cell::new(0));
    let mut source = EmptyErrorInput(Rc::clone(&calls));
    let borrowed: &mut dyn Input<Item = char> = &mut source;
    let mut input = InputRef::new(borrowed);
    let mut empty: [&mut [char]; 0] = [];
    assert_eq!(
        Input::read_vectored(&mut input, &mut empty).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls.get(), 1);
}

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
