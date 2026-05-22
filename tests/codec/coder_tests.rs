use qubit_io::{
    Coder,
    CoderProgress,
    CoderStatus,
};

#[derive(Default)]
struct CopyCoder;

impl Coder<u8, u8> for CopyCoder {
    type Error = core::convert::Infallible;

    fn max_output_len(&self, input_len: usize) -> Option<usize> {
        Some(input_len)
    }

    fn convert(
        &mut self,
        input: &[u8],
        input_index: usize,
        output: &mut [u8],
        output_index: usize,
    ) -> Result<CoderProgress, Self::Error> {
        let mut read = 0;
        let mut written = 0;
        while input_index + read < input.len() && output_index + written < output.len() {
            output[output_index + written] = input[input_index + read];
            read += 1;
            written += 1;
        }
        if input_index + read == input.len() {
            Ok(CoderProgress::complete(read, written))
        } else {
            let status = CoderStatus::NeedOutput {
                output_index: output_index + written,
                required: 1,
                available: output.len().saturating_sub(output_index + written),
            };
            Ok(CoderProgress::new(status, read, written))
        }
    }
}

#[test]
fn test_coder_contract_uses_absolute_indices_and_relative_progress() {
    let mut coder = CopyCoder;
    let mut output = [0_u8; 4];

    let progress = coder
        .convert(b"abc", 1, &mut output, 2)
        .expect("infallible copy");

    assert_eq!(CoderStatus::Complete, progress.status());
    assert_eq!(2, progress.read());
    assert_eq!(2, progress.written());
    assert_eq!([0, 0, b'b', b'c'], output);
}

#[test]
fn test_coder_default_reset_and_finish_are_noops() {
    let mut coder = CopyCoder;
    let mut output = [0_u8; 1];

    assert_eq!(Some(3), coder.max_output_len(3));

    Coder::<u8, u8>::reset(&mut coder);
    let progress = Coder::<u8, u8>::finish(&mut coder, &mut output, 0).expect("finish is noop");

    assert_eq!(CoderStatus::Complete, progress.status());
    assert_eq!(0, progress.read());
    assert_eq!(0, progress.written());
    assert_eq!([0], output);
}
