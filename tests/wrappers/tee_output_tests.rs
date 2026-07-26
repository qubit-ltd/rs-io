//! Tests for [`qubit_io::TeeOutput`].

use std::io;

use qubit_io::{
    Output,
    TeeOutput,
};

/// Output collecting generic items.
struct Items(Vec<u16>);

impl Output for Items {
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
fn test_tee_output_mirrors_generic_items() {
    let mut output = TeeOutput::new(Items(Vec::new()), Items(Vec::new()));

    assert_eq!(2, output.write(&[7_u16, 9]).expect("write should succeed"));
    assert_eq!(vec![7, 9], output.inner().0);
    assert_eq!(vec![7, 9], output.branch().0);
}
