// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Tests for [`qubit_io::CountingOutput`].

use std::io;

use qubit_io::{
    CountingOutput,
    Output,
};

/// In-memory output for non-byte item tests.
struct U16Output {
    /// Items accepted by this output.
    items: Vec<u16>,
}

impl Output for U16Output {
    /// Item type accepted by this output.
    type Item = u16;

    /// Appends each requested item.
    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    /// Has no buffered items to flush.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_counting_output_counts_successful_generic_items() {
    let mut output = CountingOutput::new(U16Output { items: Vec::new() });

    assert_eq!(
        2,
        output.write(&[13_u16, 21]).expect("write should succeed")
    );
    assert_eq!(2, output.items_written());
    assert_eq!(vec![13, 21], output.inner().items);
}
