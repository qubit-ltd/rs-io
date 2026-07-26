//! Tests for [`qubit_io::LimitInput`].

use std::io;

use qubit_io::{
    Input,
    LimitInput,
};

/// Generic input that returns a fixed item sequence.
struct InputItems(Vec<u16>);

impl Input for InputItems {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [Self::Item],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = self.0.len().min(count);
        output[index..index + read].copy_from_slice(&self.0[..read]);
        self.0.drain(..read);
        Ok(read)
    }
}

#[test]
fn test_limit_input_limits_generic_items() {
    let mut input = LimitInput::new(InputItems(vec![1, 2, 3]), 2);
    let mut items = [0_u16; 3];

    assert_eq!(2, input.read(&mut items).expect("read should succeed"));
    assert_eq!([1, 2, 0], items);
    assert_eq!(0, input.remaining());
}
