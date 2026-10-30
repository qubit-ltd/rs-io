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

use qubit_io::Output;
use qubit_io::OutputRef;

struct VectorCharOutput(Rc<RefCell<Vec<char>>>);
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
fn test_output_ref_forwards_vectored_override() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let mut sink = VectorCharOutput(Rc::clone(&values));
    let borrowed: &mut dyn Output<Item = char> = &mut sink;
    let mut output = OutputRef::new(borrowed);
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
fn test_output_ref_preserves_empty_vectored_write_error() {
    let calls = Rc::new(Cell::new(0));
    let mut sink = EmptyErrorOutput(Rc::clone(&calls));
    let borrowed: &mut dyn Output<Item = char> = &mut sink;
    let mut output = OutputRef::new(borrowed);
    assert_eq!(
        Output::write_vectored(&mut output, &[]).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls.get(), 1);
}

struct CharOutput {
    items: Vec<char>,
    buffered: bool,
    flushed: bool,
}

impl CharOutput {
    fn new(buffered: bool) -> Self {
        Self {
            items: Vec::new(),
            buffered,
            flushed: false,
        }
    }
}

impl Output for CharOutput {
    type Item = char;

    fn is_buffered(&self) -> bool {
        self.buffered
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
fn test_output_ref_forwards_writes_and_exposes_borrowed_output() {
    let mut inner = CharOutput::new(true);
    let mut output = OutputRef::new(&mut inner);

    assert!(output.is_buffered());
    assert!(output.get_ref().buffered);
    output.get_mut().buffered = false;
    assert!(!output.get_ref().buffered);

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
    assert_eq!(vec!['b', 'c', 'd', 'e'], inner.items);
    assert!(inner.flushed);
}
