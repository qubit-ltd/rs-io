// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Tests for [`qubit_io::CountingInput`].

use std::io;

use qubit_io::{
    CountingInput,
    Input,
};

/// In-memory input for non-byte item tests.
struct U16Input {
    /// Items made available to callers.
    items: Vec<u16>,
    /// Next unread item index.
    position: usize,
}

impl U16Input {
    /// Creates an input over `items`.
    fn new(items: Vec<u16>) -> Self {
        Self { items, position: 0 }
    }
}

impl Input for U16Input {
    /// Item type returned by this input.
    type Item = u16;

    /// Reads the requested available item prefix.
    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let available = self.items.len() - self.position;
        let read = available.min(count);
        output[index..index + read]
            .copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[test]
fn test_counting_input_counts_successful_generic_items() {
    let mut input = CountingInput::new(U16Input::new(vec![3, 5, 8]));
    let mut output = [0_u16; 2];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([3, 5], output);
    assert_eq!(2, input.items_read());
}
