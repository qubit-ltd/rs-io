//! Tests for [`qubit_io::LimitOutput`].

use std::io;

use qubit_io::{
    LimitOutput,
    Output,
};

/// Generic output that stores accepted items.
struct OutputItems(Vec<u16>);

impl Output for OutputItems {
    type Item = u16;

    unsafe fn write_unchecked(
        &mut self,
        input: &[Self::Item],
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
fn test_limit_output_limits_generic_items() {
    let mut output = LimitOutput::new(OutputItems(Vec::new()), 2);

    assert_eq!(
        2,
        output.write(&[1_u16, 2, 3]).expect("write should succeed")
    );
    assert_eq!(0, output.remaining());
    assert_eq!(vec![1, 2], output.inner().0);
}
