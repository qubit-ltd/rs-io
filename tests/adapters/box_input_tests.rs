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

use qubit_io::BoxInput;
use qubit_io::Input;
use qubit_io::InputRef;

struct VectorCharInput;
impl Input for VectorCharInput {
    type Item = char;
    unsafe fn read_unchecked(&mut self, _: &mut [char], _: usize, _: usize) -> io::Result<usize> {
        panic!("scalar read called")
    }
    fn read_vectored(&mut self, buffers: &mut [&mut [char]]) -> io::Result<usize> {
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
fn vectored_forward_box_input_keeps_override() {
    let inner: Box<dyn Input<Item = char>> = Box::new(VectorCharInput);
    let mut input = BoxInput::new(inner);
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

    unsafe fn read_unchecked(&mut self, _: &mut [char], _: usize, _: usize) -> io::Result<usize> {
        panic!("scalar read called")
    }

    fn read_vectored(&mut self, _: &mut [&mut [char]]) -> io::Result<usize> {
        self.0.set(self.0.get() + 1);
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}

#[test]
fn vectored_forward_box_input_preserves_empty_list_error() {
    let calls = Rc::new(Cell::new(0));
    let mut source = EmptyErrorInput(Rc::clone(&calls));
    let borrowed: &mut dyn Input<Item = char> = &mut source;
    let inner = BoxInput::new(Box::new(InputRef::new(borrowed)));
    let mut input = BoxInput::new(Box::new(inner));
    let mut empty: [&mut [char]; 0] = [];
    assert_eq!(
        Input::read_vectored(&mut input, &mut empty).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls.get(), 1);
}

struct CharInput {
    items: Vec<char>,
    position: usize,
}

impl Input for CharInput {
    type Item = char;

    fn is_buffered(&self) -> bool {
        true
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

    let mut inner = input.into_inner();
    assert_eq!(1, inner.read(&mut output[..1]).expect("read should succeed"));
    assert_eq!('e', output[0]);
}
