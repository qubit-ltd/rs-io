//! Tests for [`qubit_io::TeeInput`].

use std::io;

use qubit_io::{
    Input,
    Output,
    TeeInput,
};

/// Input with a fixed generic item sequence.
struct Items(Vec<u16>);

impl Input for Items {
    type Item = u16;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [u16],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = self.0.len().min(count);
        output[index..index + read].copy_from_slice(&self.0[..read]);
        self.0.drain(..read);
        Ok(read)
    }
}

/// Output collecting generic items.
struct Branch(Vec<u16>);

impl Output for Branch {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[u16],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        self.0.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_tee_input_mirrors_generic_items() {
    let mut input = TeeInput::new(Items(vec![4, 6]), Branch(Vec::new()));
    let mut output = [0_u16; 2];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([4, 6], output);
    assert_eq!(vec![4, 6], input.branch().0);
}
