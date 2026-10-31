// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use qubit_io::Input;
use qubit_io::Output;
use qubit_io::Seekable;

pub(super) struct CharStream {
    pub(super) items: [char; 3],
    position: usize,
}

impl CharStream {
    #[must_use]
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            items: ['你', '🦀', '\u{0301}'],
            position: 0,
        }
    }
}

impl Input for CharStream {
    type Item = char;

    unsafe fn read_unchecked(&mut self, output: &mut [char], index: usize, count: usize) -> io::Result<usize> {
        let count = count.min(self.items.len() - self.position);
        output[index..index + count].copy_from_slice(&self.items[self.position..self.position + count]);
        self.position += count;
        Ok(count)
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
