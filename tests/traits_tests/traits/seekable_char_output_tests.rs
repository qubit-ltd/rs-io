// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use qubit_io::CountingOutput;
use qubit_io::Output;
use qubit_io::Seekable;
use qubit_io::SeekableCharOutput;
use qubit_io::SeekableOutput;

struct CharStream {
    items: [char; 3],
    position: usize,
}

impl CharStream {
    fn new() -> Self {
        Self {
            items: ['你', '🦀', '\u{0301}'],
            position: 0,
        }
    }
}

impl Output for CharStream {
    type Item = char;

    unsafe fn write_unchecked(&mut self, input: &[char], index: usize, count: usize) -> io::Result<usize> {
        let count = count.min(self.items.len() - self.position);
        self.items[self.position..self.position + count].copy_from_slice(&input[index..index + count]);
        self.position += count;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seekable for CharStream {
    type Unit = char;

    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => self.position as i128 + i128::from(offset),
            SeekFrom::End(offset) => self.items.len() as i128 + i128::from(offset),
        };
        if !(0..=self.items.len() as i128).contains(&position) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid character position",
            ));
        }
        self.position = position as usize;
        Ok(position as u64)
    }
}

fn transfer<S: SeekableCharOutput + ?Sized>(stream: &mut S) {
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("seek should succeed"), 1);
    stream.write_fully(&['好']).expect("write should succeed");
    assert_eq!(stream.seek_to(SeekFrom::Current(0)).expect("query should succeed"), 2);
}

#[test]
fn test_parameter_and_opaque_return() {
    fn stream() -> impl SeekableCharOutput {
        CharStream::new()
    }
    transfer(&mut stream());
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_bound() {
    let mut stream = CharStream::new();
    let stream: &mut dyn SeekableOutput<Item = char, Unit = char> = &mut stream;
    transfer(stream);
}

#[test]
fn test_trait_object_preserves_wrapper_seek_capability() {
    let mut stream: Box<dyn SeekableCharOutput> = Box::new(CountingOutput::new(CharStream::new()));
    transfer(stream.as_mut());
    assert_eq!(stream.seek_to(SeekFrom::Start(1)).expect("rewind should succeed"), 1);
    stream.write_fully(&['好']).expect("overwrite should succeed");
}

#[test]
fn test_seek_overwrites_one_char() {
    let mut stream = CharStream::new();
    transfer(&mut stream);
    assert_eq!(stream.items, ['你', '好', '\u{0301}']);
}
