// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::Cell;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use qubit_io::BoxOutput;
use qubit_io::Output;
use qubit_io::OutputRef;

struct VectorCharOutput(Rc<RefCell<Vec<char>>>);

// Rejects scalar writes so the test verifies vectored-write forwarding.
impl Output for VectorCharOutput {
    type Item = char;
    unsafe fn write_unchecked(&mut self, _: &[char], _: usize, _: usize) -> io::Result<usize> {
        panic!("scalar write called")
    }
    fn write_vectored(&mut self, buffers: &[&[char]]) -> io::Result<usize> {
        let mut total = 0;
        for buffer in buffers {
            self.0.borrow_mut().extend_from_slice(buffer);
            total += buffer.len();
        }
        Ok(total)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_forward_box_output_keeps_override() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let inner: Box<dyn Output<Item = char>> = Box::new(VectorCharOutput(Rc::clone(&values)));
    let mut output = BoxOutput::new(inner);
    assert_eq!(
        Output::write_vectored(&mut output, &[&['甲', '乙'], &['丙', '丁']]).unwrap(),
        4
    );
    assert_eq!(*values.borrow(), ['甲', '乙', '丙', '丁']);
}

struct EmptyErrorOutput(Rc<Cell<usize>>);

impl Output for EmptyErrorOutput {
    type Item = char;

    unsafe fn write_unchecked(&mut self, _: &[char], _: usize, _: usize) -> io::Result<usize> {
        panic!("scalar write called")
    }

    fn write_vectored(&mut self, _: &[&[char]]) -> io::Result<usize> {
        self.0.set(self.0.get() + 1);
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_forward_box_output_preserves_empty_list_error() {
    let calls = Rc::new(Cell::new(0));
    let mut sink = EmptyErrorOutput(Rc::clone(&calls));
    let borrowed: &mut dyn Output<Item = char> = &mut sink;
    let inner = BoxOutput::new(Box::new(OutputRef::new(borrowed)));
    let mut output = BoxOutput::new(Box::new(inner));
    assert_eq!(
        Output::write_vectored(&mut output, &[]).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls.get(), 1);
}

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
